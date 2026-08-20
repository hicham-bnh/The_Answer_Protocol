# The Answer Protocol  a 42TAP MUD

*This project has been created as part of the 42 curriculum by mdourdoi>, mobenhab>.*

## Description

A multiplayer text-based MUD (Multi-User Dungeon) built on the 42TAP protocol:
a TCP server hosting a vanilla shared fantasy world where players explore, chat, fight
NPCs, complete quests and form groups together and in real time plus a
graphical client (egui) and a command-line client. Written in Rust, with the
standard library only for all networking, and egui for the GUI.

The GUI is intentionnaly very basic (no avatar, no images) to fit the vibe of old school MUD,
but is created in a way that everything gets updated at any time.

## Instructions

The server always run on port 8080. Make sure this port is available when lauching the server.
Quick start (details in Building and Running below):

```
make install && make run-server        # terminal 1
make run-client-gui                    # terminal 2 (and 3, 4... for more players)
```

## Architecture

The workspace contains three crates:

- `crates/server`  multithreaded TCP server (one thread per client), owner of
  all game state. Command handling is inline: each client thread parses its
  line and dispatches through a single `match` in `parse_command`  no separate
  router layer, a deliberate simplicity choice for a fixed command set. Shared state lives behind `Arc<Mutex<...>>`: the game world
  (rooms, NPCs, items, quests) and the connected-players map. Lock ordering is
  consistent everywhere (world before players) to prevent deadlocks.
- `crates/client_gui`  graphical client built with eframe/egui. Two background
  threads own the socket (a line reader and a writer), connected to the UI
  through mpsc channels; the UI thread never blocks on the network.
- `crates/client_cli`  minimal command-line client: raw protocol lines in,
  raw protocol lines out (see Building and Running).

Server and clients deliberately do not share a crate. Each side defines its own
mirror types for the JSON payloads it produces or consumes. This keeps the two
halves independently compilable against any conforming 42TAP implementation and
makes the protocol, not shared Rust code, the single source of truth. Mirror
structs describe the full wire format; fields the code does not currently read
are kept and marked `#[allow(dead_code)]` as living documentation.

The GUI reconciles server replies through a FIFO pending-command queue: the
protocol guarantees one reply per command, in order, so each incoming `OK`/`ERR`
is matched with the oldest in-flight command. Asynchronous `EVT` lines bypass
the queue entirely.

## Protocol Implementation

The server implements the 42TAP RFC: greeting `OK hello proto=1`, line-based
commands, `OK`/`ERR` replies and asynchronous `EVT` broadcasts (room presence,
chat on the three scopes, group events, server stats). Structured reply payloads
are JSON, serialized with serde.

Documented deviations and extensions:

- `QUEST`/`QUESTS` replies include two extra fields, `name` and `description`,
  taken from the world definition. Clients that ignore unknown JSON fields are
  unaffected; our GUI treats them as optional and falls back to the quest id.
- Items and NPCs can be referenced by id (`item.ale`) or by display name
  (`Frothy Ale`), case-insensitively, in `TAKE`, `DROP`, `TALK`, `ATTACK` and
  `QUEST`.
- `TALK` dialogue rotation is tracked per player, so every player hears an
  NPC's lines from the beginning regardless of other players' conversations.
- Error codes. RFC codes are reused wherever they exist; the following are
  project-specific (or reuse a number with a second message, like the RFC's own
  dual `404`):
  - `101 ALREADY_CONNECTED`  second `CONNECT` on the same session
  - `405 ITEM_NOT_OBTAINABLE`  item present but flagged non-obtainable
  - `407 NPC_BUSY`  NPC already fighting another player
  - `407 NOT_IN_COMBAT`  `ATTACK`/`DEFEND`/`FLEE` outside combat
  - `409 IN_COMBAT`  non-combat command while fighting
  - `412 QUEST_ALREADY_COMPLETED`  asking for an already-delivered quest
  - `500 INTERNAL_ERROR`  unexpected server-side failure
- Client-side only: when the connection dies, the GUI synthesizes
  `ERR 900 DISCONNECTED` internally to switch to a terminal "disconnected"
  screen. Nothing is sent on the wire. There is no automatic reconnection:
  restart the client (a deliberate simplification since nothing in the subject
  requires session resumption).

## Combat System

Turn-based 1-versus-1 combat, player-initiated:

- `ATTACK npc.x` engages if the NPC is present, hostile, alive and free. A
  busy NPC answers `407 NPC_BUSY`: one fighter at a time (`engaged_by` lock).
- Each round is player-first: `ATTACK` deals 15 damage, then the NPC
  counter-attacks with its yaml-defined damage. `DEFEND` deals no damage and
  halves the incoming counter-attack. `FLEE` always succeeds and is free (no
  parting shot)  a documented simplification.
- Outcomes: NPC at 0 HP → `won`, NPC removed from the room and respawned at
  full HP after its `respawn_seconds` (default 30). Player at 0 HP → `dead`,
  respawned at the respawn location with 50 HP. All transitions (engage,
  victory, death, flight) are logged and broadcast to the other players in the
  room as room-chat messages, so spectators see fights unfold in real time.
- NPC damage persists across engagements: a wounded NPC stays wounded if its
  attacker disconnects, dies or flees; only its post-victory respawn restores
  full HP. This is the shared-world behaviour, kept deliberately.
- A player disconnecting mid-fight releases the NPC's combat lock immediately.

The GUI switches the room panel to a combat view while fighting: enemy health
bar, action buttons and a scrolling combat log; the player's own health bar
lives in the top bar permanently.

## Quest System

Two quest types are implemented end to end: fetch (bring an item to an NPC)
and kill (defeat N enemies). `QUEST npc.x` is context-sensitive: it accepts an
offered quest, reports progress, or completes the quest and delivers the reward
into the player's inventory. Completion is validated server-side: a fetch
quest checks the required item is actually in the player's inventory (and
consumes it), a kill quest counts defeated targets in the player's progress
map. `QUESTS` lists the player's journal (in-progress
first, then completed) with `name`, `description`, `status` and `progress`.

Client-side, the journal is refreshed generically: after every successful
command the GUI re-issues `QUESTS` (suppressed while in combat and
after `QUESTS` itself to avoid loops). This makes progress tracking independent
of which server mechanic advances a quest, so the client works unchanged
against worlds where quests advance on movement, dialogue or anything else.
Completing a quest also triggers an automatic `INVENTORY` refresh to show the
reward.

## World Design

The world is defined in `config/world.yaml`: 9 interconnected rooms with loops:

```
           [tavern]
              |
  [forge] - [square] - [market]
     |         |          |
  [gate] -- [road] --- [forest]
               |
            [cave] --- [crypt]
```

4 NPCs in
3 roles (dialogue guard, two quest givers, one hostile enemy), 7 items  4
obtainable ones placed in rooms, one non-obtainable piece of scenery (the
ancient altar), two quest rewards  and 2 quests (one fetch, one kill).

At startup the server validates the whole file before serving: unreadable file
or invalid YAML aborts with a clear error; then every reference is checked 
start/respawn locations, every exit destination, every placed item and NPC,
every quest offered by an NPC, and each quest's giver, objective and reward.
All problems are listed at once and the server refuses to start. By
construction, no invalid room or dangling reference can ever be assigned at
runtime. NPCs without an explicit `max_hp` inherit their starting `hp`.

## Server Logging

Logs are single-line JSON on stdout with a Unix timestamp (`ts`), a level and
an event name: connections opened/closed with peer IP, every command received,
combat events, disconnects, and errors. Suspicious activity is detected and
logged: more than 100 commands within 5 seconds raises a `flood_detected`
warning naming the player. Monitoring is a matter of watching stdout or
redirecting it (`make run-server > server.log`, then `grep WARN` or any JSON
tool). Flooding is log-only by design  flooded commands
are still answered, because silently dropping a command would break the
protocol's one-reply-per-command guarantee that clients rely on.

The server holds no persistent state; stopping it with Ctrl+C is the intended
shutdown and loses nothing. Clients detect the closed connection and report it.

## Group Contributions

- mdourdoi > graphical client (`client_gui`): egui interface, networking
  threads, protocol parser and mirror types, combat/quest/group UI, automated
  test suite; command-line client (`client_cli`), rewritten on the GUI's
  network architecture; world design review and cross-testing.
- mobenhab >  server (`server`): TCP loop, world loading and validation,
  command handling, combat, quests, groups, chat broadcasting, JSON logging
  and flood detection; `config/world.yaml`.
- Shared: protocol decisions, error-code catalogue, combat design document,
  this README, debugging sessions with both halves running against each other.

## Building and Running

Rust stable and cargo are the only requirements.

```
make install          # cargo build (fetches and compiles everything)
make run-server       # serves on 127.0.0.1:8080
make run-client-gui   # GUI client        (default ARGS=127.0.0.1:8080)
make run-client       # CLI client        (default ARGS=127.0.0.1:8080)
make test             # cargo test --workspace
make lint             # clippy -D warnings + rustfmt --check
make clean
```

Both clients take the server address as their single argument
(`make run-client-gui ARGS=10.0.1.5:8080`). The address must be a literal
`ip:port`; `localhost` is rejected on purpose (use `127.0.0.1`). The CLI is a
transparent protocol console: type raw 42TAP commands, sent lines are echoed
with `>>`, received lines printed with `<<`  reading events while waiting for
input is guaranteed by its two-thread design. It also works non-interactively
(`echo "QUIT" | client_cli 127.0.0.1:8080`).

## Testing

`make test` runs the automated suite: 50 unit tests in the GUI crate: 7 for
the line parser (OK/ERR/EVT variants, malformed input) and 43 for the state
machine (command/reply reconciliation, JSON mirrors, combat rounds and all
exit transitions, quest journal upserts, group invitations, unsolicited and
malformed replies, disconnection). Tests drive the client through the public
protocol only, with no network, by injecting server lines.

Manual test plan used before delivery: two GUI clients plus the CLI against
the server (chat scopes and ordering, presence, shared items, group flow,
combat with spectators, quest cycle, mid-combat disconnection, server kill,
flood attempt via `yes | client_cli`), plus a sabotaged `world.yaml` to check
startup validation, and cross-testing against another group's server and
clients.

## Resources

- 42TAP protocol RFC and project subject (42)
- The Rust Programming Language (the Book), std documentation
- serde / serde_json / serde_yaml documentation
- egui / eframe documentation (docs.rs)

AI usage: an AI assistant (Claude) was used throughout the project as helper with Rust syntax and documentation for example with egui concepts, and for reviewing code.