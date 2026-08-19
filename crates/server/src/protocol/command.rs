use crate::world_struct::world_struct::GameWorld;
use crate::Player;
use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
struct RoomInfo {
    id: String,
    name: String,
    description: String,
    exits: HashMap<String, String>,
}

#[derive(Serialize)]
struct LookReply {
    room: RoomInfo,
    players: Vec<String>,
    items: Vec<String>,
    npcs: Vec<String>,
}

#[derive(Serialize)]
struct StatusReply {
    hp: u32,
    max_hp: u32,
    status: String,
}

#[derive(Serialize)]
struct WhoReply {
    room: Vec<String>,
    server: u32,
}

#[derive(Serialize)]
struct TalkReply {
    npc: String,
    dialogue: String,
}

#[derive(Serialize)]
struct CombatReply {
    action: String,
    attacker_hp: u32,
    target_hp: u32,
    damage_dealt: u32,
    damage_taken: u32,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Serialize)]
struct QuestReply {
    quest_id: String,
    name: String,
    description: String,
    status: String,
    progress: String,
}

fn send_err(stream: &mut TcpStream, code: &str) {
    let _ = stream.write_all(format!("ERR {}\n", code).as_bytes());
}

pub fn log(level: &str, event: &str, details: serde_json::Value) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let entry = json!({
        "ts": ts,
        "level": level,
        "event": event,
        "details": details,
    });
    println!("{}", entry);
}

fn broadcast_room(
    guard: &mut HashMap<String, Player>,
    room: &str,
    msg: &str,
    exclude: Option<&str>,
) {
    for (client_name, client) in guard.iter_mut() {
        if client.room == room && Some(client_name.as_str()) != exclude {
            let _ = client.stream.lock().unwrap().write_all(msg.as_bytes());
        }
    }
}

fn broadcast_all(guard: &mut HashMap<String, Player>, msg: &str, exclude: Option<&str>) {
    for (client_name, client) in guard.iter_mut() {
        if Some(client_name.as_str()) != exclude {
            let _ = client.stream.lock().unwrap().write_all(msg.as_bytes());
        }
    }
}

fn find_npc_in_room(w: &GameWorld, room: &str, npc_arg: &str) -> Option<String> {
    let loc = w.world.locations.get(room)?;
    loc.npcs
        .iter()
        .find(|id| {
            id.as_str() == npc_arg
                || w.npcs
                    .get(id.as_str())
                    .map(|n| n.name.eq_ignore_ascii_case(npc_arg))
                    .unwrap_or(false)
        })
        .cloned()
}

fn apply_take_quest_progress(player: &mut Player, w: &GameWorld, item_id: &str) {
    for qid in player.quest_to_do.clone() {
        if let Some(quest) = w.quests.get(&qid) {
            if quest.objective.item.as_deref() == Some(item_id) {
                let total = quest.objective.count.unwrap_or(1);
                let entry = player.quest_progress.entry(qid).or_insert(0);
                if *entry < total {
                    *entry += 1;
                }
            }
        }
    }
}

fn apply_kill_quest_progress(player: &mut Player, w: &GameWorld, npc_id: &str) {
    for qid in player.quest_to_do.clone() {
        if let Some(quest) = w.quests.get(&qid) {
            if quest.objective.target.as_deref() == Some(npc_id) {
                let total = quest.objective.count.unwrap_or(1);
                let entry = player.quest_progress.entry(qid).or_insert(0);
                if *entry < total {
                    *entry += 1;
                }
            }
        }
    }
}

pub fn connect_user(
    line: &str,
    stream: &mut TcpStream,
    players: &Arc<Mutex<HashMap<String, Player>>>,
    spawn_room: String,
) -> (bool, String) {
    let mut args = line.splitn(2, ' ');
    match args.next() {
        Some("CONNECT") => {
            let Some(arg_name) = args.next() else {
                send_err(stream, "400 MALFORMED_COMMAND");
                return (false, String::new());
            };
            let arg_name = arg_name.trim();
            if arg_name.is_empty() || arg_name.contains(char::is_whitespace) {
                send_err(stream, "400 MALFORMED_COMMAND");
                return (false, String::new());
            }
            let name = arg_name.to_string();

            let mut guard = players.lock().unwrap();
            if guard.contains_key(&name) {
                send_err(stream, "201 NAME_IN_USE");
                return (false, String::new());
            }

            let stream_for_map = match stream.try_clone() {
                Ok(s) => s,
                Err(_) => {
                    send_err(stream, "500 INTERNAL_ERROR");
                    return (false, String::new());
                }
            };

            let player = Player {
                stream: Mutex::new(stream_for_map),
                room: spawn_room.to_string(),
                name: name.clone(),
                inventory: vec![],
                hp: 100,
                max_hp: 100,
                pv: 15,
                combat: "healthy".to_string(),
                quest_to_do: vec![],
                quest_dine: vec![],
                quest_progress: HashMap::new(),
                target_npc: None,
                group: None,
                invite_from: None,
                dialogue_progress: HashMap::new(),
            };
            guard.insert(name.clone(), player);
            let _ = stream.write_all(b"OK connected\n");
            let enter_evt = format!("EVT ROOM PRESENCE ENTER {}\n", name);
            broadcast_room(&mut guard, &spawn_room, &enter_evt, Some(&name));
            let stats_evt = format!("EVT STATS players={}\n", guard.len());
            broadcast_all(&mut guard, &stats_evt, None);

            log(
                "INFO",
                "player_connected",
                json!({"player": name, "room": spawn_room}),
            );
            (true, name)
        }
        Some("QUIT") => {
            let _ = stream.write_all(b"OK bye\n");
            log("INFO", "quit_before_connect", json!({}));
            (false, String::new())
        }
        Some(cmd_inconnue) => {
            send_err(stream, "100 NOT_CONNECTED");
            log(
                "WARN",
                "command_before_connect",
                json!({"command": cmd_inconnue}),
            );
            (false, String::new())
        }
        None => {
            send_err(stream, "400 MALFORMED_COMMAND");
            (false, String::new())
        }
    }
}

pub fn leave_group(name: &str, players: &Arc<Mutex<HashMap<String, Player>>>) {
    let mut guard = players.lock().unwrap();
    let my_group = match guard.get(name).and_then(|p| p.group.clone()) {
        Some(g) => g,
        None => return,
    };
    if let Some(player) = guard.get_mut(name) {
        player.group = None;
    }
    let msg_evt = format!("EVT GROUP LEAVE {}\n", name);
    for (client_name, client_stream) in guard.iter_mut() {
        if client_name != name && client_stream.group.as_deref() == Some(my_group.as_str()) {
            let _ = client_stream
                .stream
                .lock()
                .unwrap()
                .write_all(msg_evt.as_bytes());
        }
    }
}

pub fn disconnect_player(name: &str, players: &Arc<Mutex<HashMap<String, Player>>>) {
    if name.is_empty() {
        return;
    }
    let room = {
        let mut guard = players.lock().unwrap();
        let room = guard.get(name).map(|p| p.room.clone());
        guard.remove(name);
        room
    };
    {
        let mut guard = players.lock().unwrap();
        if let Some(room) = &room {
            let leave_evt = format!("EVT ROOM PRESENCE LEAVE {}\n", name);
            broadcast_room(&mut guard, room, &leave_evt, None);
        }
        let stats_evt = format!("EVT STATS players={}\n", guard.len());
        broadcast_all(&mut guard, &stats_evt, None);
    }
    leave_group(name, players);
    log("INFO", "player_disconnected", json!({"player": name}));
}

pub fn parse_command(
    line: &str,
    stream: &mut TcpStream,
    players: &Arc<Mutex<HashMap<String, Player>>>,
    name: &str,
    world: &Arc<Mutex<GameWorld>>,
    next_group_id: &Arc<Mutex<u32>>,
) {
    let mut args = line.splitn(2, ' ');
    let cmd = args.next();
    if let Some(cmd_name) = cmd {
        let in_combat = {
            let guard = players.lock().unwrap();
            guard
                .get(name)
                .map(|p| p.combat == "in_combat")
                .unwrap_or(false)
        };
        let allowed = matches!(cmd_name, "ATTACK" | "DEFEND" | "FLEE" | "STATUS" | "CHAT");
        if in_combat && !allowed {
            send_err(stream, "409 IN_COMBAT");
            log(
                "WARN",
                "command_rejected_in_combat",
                json!({"player": name, "command": cmd_name}),
            );
            return;
        }
    }
    match cmd {
        Some("CONNECT") => {
            send_err(stream, "101 ALREADY_CONNECTED");
        }
        Some("LOOK") => {
            let mut guard = players.lock().unwrap();
            let room_str = match guard.get(name) {
                Some(player) => player.room.clone(),
                None => return,
            };
            let mut players_list: Vec<String> = Vec::new();
            for (client_name, client_steam) in guard.iter_mut() {
                if client_steam.room == room_str {
                    players_list.push(client_name.to_string());
                }
            }
            if let Some(player) = guard.get(name) {
                let w = world.lock().unwrap();
                let loc = match w.world.locations.get(&room_str) {
                    Some(l) => l,
                    None => return,
                };
                let reply = LookReply {
                    room: RoomInfo {
                        id: room_str.clone(),
                        name: loc.name.clone(),
                        description: loc.description.clone(),
                        exits: loc.exits.clone(),
                    },
                    players: players_list,
                    items: loc.items.clone(),
                    npcs: loc.npcs.clone(),
                };
                let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
                let mut guard_stream = player.stream.lock().unwrap();
                let _ = guard_stream.write_all(msg.as_bytes());
            }
            log("INFO", "look", json!({"player": name, "room": room_str}));
        }
        Some("MOVE") => {
            let Some(rest) = args.next() else {
                send_err(stream, "400 MALFORMED_COMMAND");
                return;
            };
            let mut chat_args = rest.splitn(2, ' ');
            let move_to = chat_args.next().unwrap_or("");
            let mut guard = players.lock().unwrap();
            if let Some(player) = guard.get_mut(name) {
                let room_str = player.room.clone();
                let w = world.lock().unwrap();
                let exits = &w.world.locations.get(&room_str).unwrap().exits;
                if let Some(room_to) = exits.get(move_to) {
                    let room_to = room_to.clone();
                    let msg = format!("OK room={}\n", room_to);
                    let _ = stream.write_all(msg.as_bytes());
                    player.room = room_to.clone();

                    let leave_evt = format!("EVT ROOM PRESENCE LEAVE {}\n", name);
                    let enter_evt = format!("EVT ROOM PRESENCE ENTER {}\n", name);
                    for (_client_name, client_steam) in guard.iter_mut() {
                        if client_steam.room == room_str {
                            let _ = client_steam
                                .stream
                                .lock()
                                .unwrap()
                                .write_all(leave_evt.as_bytes());
                        } else if client_steam.room == room_to {
                            let _ = client_steam
                                .stream
                                .lock()
                                .unwrap()
                                .write_all(enter_evt.as_bytes());
                        }
                    }
                    log(
                        "INFO",
                        "player_moved",
                        json!({"player": name, "from": room_str, "to": room_to}),
                    );
                } else {
                    send_err(stream, "301 NO_EXIT");
                }
            }
        }
        Some("QUIT") => {
            let _ = stream.write_all(b"OK bye\n");
            log("INFO", "quit", json!({"player": name}));
        }
        Some("CHAT") => {
            let Some(rest) = args.next() else {
                send_err(stream, "400 MALFORMED_COMMAND");
                return;
            };
            let mut chat_args = rest.splitn(2, ' ');
            let sub_chat = chat_args.next();
            if sub_chat == Some("GLOBAL") {
                let chat_msg = chat_args.next().unwrap_or("");
                let mut guard = players.lock().unwrap();
                let _ = stream.write_all(b"OK\n");
                let evt = format!("EVT GLOBAL CHAT {} {}\n", name, chat_msg);
                broadcast_all(&mut guard, &evt, None);
            } else if sub_chat == Some("GROUP") {
                let chat_msg = chat_args.next().unwrap_or("");
                let mut guard = players.lock().unwrap();
                let my_group = guard.get(name).and_then(|p| p.group.clone());
                match my_group {
                    None => send_err(stream, "401 NOT_IN_GROUP"),
                    Some(my_group) => {
                        let _ = stream.write_all(b"OK\n");
                        let evt = format!("EVT GROUP CHAT {} {}\n", name, chat_msg);
                        for (_client_name, client_steam) in guard.iter_mut() {
                            if client_steam.group.as_deref() == Some(my_group.as_str()) {
                                let _ = client_steam
                                    .stream
                                    .lock()
                                    .unwrap()
                                    .write_all(evt.as_bytes());
                            }
                        }
                    }
                }
            } else if sub_chat == Some("ROOM") {
                let chat_msg = chat_args.next().unwrap_or("");
                let mut guard = players.lock().unwrap();
                let room = guard.get(name).map(|p| p.room.clone()).unwrap_or_default();
                let _ = stream.write_all(b"OK\n");
                let evt = format!("EVT ROOM CHAT {} {}\n", name, chat_msg);
                broadcast_room(&mut guard, &room, &evt, None);
            } else {
                send_err(stream, "400 MALFORMED_COMMAND");
            }
            log("INFO", "chat", json!({"player": name}));
        }
        Some("WHO") => {
            let guard = players.lock().unwrap();
            let room = guard.get(name).map(|p| p.room.clone()).unwrap_or_default();
            let room_players: Vec<String> = guard
                .iter()
                .filter(|(_, p)| p.room == room)
                .map(|(n, _)| n.clone())
                .collect();
            let server = guard.len() as u32;
            let reply = WhoReply {
                room: room_players,
                server,
            };
            let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
            let _ = stream.write_all(msg.as_bytes());
            log("INFO", "who", json!({"player": name}));
        }
        Some("GROUP") => {
            if let Some(rest) = args.next() {
                let mut grp_args = rest.splitn(2, ' ');
                match grp_args.next() {
                    Some("CREATE") => {
                        let already_in_group = {
                            let guard = players.lock().unwrap();
                            guard.get(name).map(|p| p.group.is_some()).unwrap_or(false)
                        };
                        if already_in_group {
                            send_err(stream, "402 ALREADY_IN_GROUP");
                            return;
                        }
                        let new_id = {
                            let mut counter = next_group_id.lock().unwrap();
                            let id = format!("grp_{}", *counter);
                            *counter += 1;
                            id
                        };
                        {
                            let mut guard = players.lock().unwrap();
                            if let Some(player) = guard.get_mut(name) {
                                player.group = Some(new_id.clone());
                            }
                        }
                        let msg_success = format!("OK group={}\n", new_id);
                        let _ = stream.write_all(msg_success.as_bytes());
                    }
                    Some("INVITE") => {
                        let name_player = grp_args.next();
                        let name_ply = match name_player {
                            Some(n) if !n.trim().is_empty() => n,
                            _ => {
                                send_err(stream, "400 MALFORMED_COMMAND");
                                return;
                            }
                        };
                        let my_group = {
                            let guard = players.lock().unwrap();
                            guard.get(name).and_then(|p| p.group.clone())
                        };
                        if my_group.is_none() {
                            send_err(stream, "401 NOT_IN_GROUP");
                            return;
                        }
                        {
                            let mut guard = players.lock().unwrap();
                            if !guard.contains_key(name_ply) {
                                send_err(stream, "404 PLAYER_NOT_FOUND");
                                return;
                            }
                            if let Some(target) = guard.get_mut(name_ply) {
                                target.invite_from = Some(name.to_string());
                                let msg_evt = format!("EVT GROUP INVITE {}\n", name);
                                let _ = target.stream.lock().unwrap().write_all(msg_evt.as_bytes());
                            }
                        }
                        let _ = stream.write_all(b"OK\n");
                    }
                    Some("JOIN") => {
                        let leader_name = grp_args.next();
                        let leader = match leader_name {
                            Some(n) if !n.trim().is_empty() => n,
                            _ => {
                                send_err(stream, "400 MALFORMED_COMMAND");
                                return;
                            }
                        };
                        let already_in_group = {
                            let guard = players.lock().unwrap();
                            guard.get(name).map(|p| p.group.is_some()).unwrap_or(false)
                        };
                        if already_in_group {
                            send_err(stream, "402 ALREADY_IN_GROUP");
                            return;
                        }
                        let invited = {
                            let guard = players.lock().unwrap();
                            guard
                                .get(name)
                                .map(|p| p.invite_from.as_deref() == Some(leader))
                                .unwrap_or(false)
                        };
                        if !invited {
                            send_err(stream, "403 NOT_INVITED");
                            return;
                        }
                        let leader_group = {
                            let guard = players.lock().unwrap();
                            guard.get(leader).and_then(|p| p.group.clone())
                        };
                        let leader_group = match leader_group {
                            Some(g) => g,
                            None => {
                                send_err(stream, "404 PLAYER_NOT_FOUND");
                                return;
                            }
                        };
                        {
                            let mut guard = players.lock().unwrap();
                            if let Some(player) = guard.get_mut(name) {
                                player.invite_from = None;
                                player.group = Some(leader_group.clone());
                            }
                        }
                        let msg_success = format!("OK group={}\n", leader_group);
                        let _ = stream.write_all(msg_success.as_bytes());
                        {
                            let mut guard = players.lock().unwrap();
                            let msg_evt = format!("EVT GROUP JOIN {}\n", name);
                            for (client_name, client_stream) in guard.iter_mut() {
                                if client_name != name
                                    && client_stream.group.as_deref() == Some(leader_group.as_str())
                                {
                                    let _ = client_stream
                                        .stream
                                        .lock()
                                        .unwrap()
                                        .write_all(msg_evt.as_bytes());
                                }
                            }
                        }
                    }
                    Some("LEAVE") => {
                        let my_group = {
                            let guard = players.lock().unwrap();
                            guard.get(name).and_then(|p| p.group.clone())
                        };
                        if my_group.is_none() {
                            send_err(stream, "401 NOT_IN_GROUP");
                            return;
                        }
                        leave_group(name, players);
                        let _ = stream.write_all(b"OK\n");
                    }
                    Some(_) | None => {
                        send_err(stream, "400 MALFORMED_COMMAND");
                    }
                }
            } else {
                send_err(stream, "400 MALFORMED_COMMAND");
            }
        }
        Some("TAKE") => {
            let Some(rest) = args.next() else {
                send_err(stream, "400 MALFORMED_COMMAND");
                return;
            };
            let input = rest.trim();
            if input.is_empty() {
                send_err(stream, "400 MALFORMED_COMMAND");
                return;
            }

            let mut guard = players.lock().unwrap();
            if let Some(player) = guard.get_mut(name) {
                let room_player = player.room.clone();
                let mut w = world.lock().unwrap();

                let target_id: Option<String> = if w.items.contains_key(input) {
                    Some(input.to_string())
                } else {
                    w.items
                        .iter()
                        .find(|(_id, item)| item.name.eq_ignore_ascii_case(input))
                        .map(|(id, _item)| id.clone())
                };

                let Some(item_id) = target_id else {
                    send_err(stream, "404 ITEM_NOT_FOUND");
                    return;
                };

                let in_room = w
                    .world
                    .locations
                    .get(&room_player)
                    .map(|loc| loc.items.contains(&item_id))
                    .unwrap_or(false);

                if !in_room {
                    send_err(stream, "404 ITEM_NOT_FOUND");
                    return;
                }

                let is_obtainable = w.items.get(&item_id).map(|i| i.obtainable).unwrap_or(false);
                if !is_obtainable {
                    send_err(stream, "405 ITEM_NOT_OBTAINABLE");
                    return;
                }

                if let Some(location) = w.world.locations.get_mut(&room_player) {
                    if let Some(index) = location.items.iter().position(|x| x == &item_id) {
                        location.items.remove(index);
                        player.inventory.push(item_id.clone());
                        apply_take_quest_progress(player, &w, &item_id);
                        let msg = format!("OK taken={}\n", item_id);
                        let _ = stream.write_all(msg.as_bytes());
                        log(
                            "INFO",
                            "item_taken",
                            json!({"player": name, "item": item_id}),
                        );
                    } else {
                        send_err(stream, "404 ITEM_NOT_FOUND");
                    }
                } else {
                    send_err(stream, "404 ITEM_NOT_FOUND");
                }
            }
        }
        Some("DROP") => {
            let Some(rest) = args.next() else {
                send_err(stream, "400 MALFORMED_COMMAND");
                return;
            };
            let input = rest.trim();
            if input.is_empty() {
                send_err(stream, "400 MALFORMED_COMMAND");
                return;
            }

            let mut guard = players.lock().unwrap();
            if let Some(player) = guard.get_mut(name) {
                let room_player = player.room.clone();
                let mut w = world.lock().unwrap();

                let found_index = player.inventory.iter().position(|inv_item_id| {
                    inv_item_id == input
                        || w.items
                            .get(inv_item_id)
                            .map(|i| i.name.eq_ignore_ascii_case(input))
                            .unwrap_or(false)
                });

                if let Some(index) = found_index {
                    let item_id = player.inventory.remove(index);
                    if let Some(location) = w.world.locations.get_mut(&room_player) {
                        location.items.push(item_id.clone());
                        let msg = format!("OK dropped={}\n", item_id);
                        let _ = stream.write_all(msg.as_bytes());
                        log(
                            "INFO",
                            "item_dropped",
                            json!({"player": name, "item": item_id}),
                        );
                    } else {
                        send_err(stream, "404 ITEM_NOT_FOUND");
                    }
                } else {
                    send_err(stream, "404 ITEM_NOT_IN_INVENTORY");
                }
            }
        }
        Some("INVENTORY") => {
            let guard = players.lock().unwrap();
            if let Some(player) = guard.get(name) {
                let msg = format!("OK {}\n", serde_json::to_string(&player.inventory).unwrap());
                let _ = stream.write_all(msg.as_bytes());
            } else {
                send_err(stream, "404 PLAYER_NOT_FOUND");
            }
        }
        Some("TALK") => {
            let Some(rest) = args.next() else {
                send_err(stream, "400 MALFORMED_COMMAND");
                return;
            };
            let npc_arg = rest.trim();
            let room = {
                let guard = players.lock().unwrap();
                guard.get(name).map(|p| p.room.clone())
            };
            let Some(room) = room else {
                return;
            };

            let mut w = world.lock().unwrap();
            let Some(npc_id) = find_npc_in_room(&w, &room, npc_arg) else {
                send_err(stream, "404 NPC_NOT_FOUND");
                return;
            };

            let dialogue = match w.npcs.get(&npc_id) {
                Some(n) if !n.dialogue.is_empty() => {
                    let mut guard = players.lock().unwrap();
                    let idx = match guard.get_mut(name) {
                        Some(p) => {
                            let counter = p.dialogue_progress.entry(npc_id.clone()).or_insert(0);
                            let idx = *counter % n.dialogue.len();
                            *counter = counter.wrapping_add(1);
                            idx
                        }
                        None => 0,
                    };
                    Some(n.dialogue[idx].clone())
                }
                _ => None,
            };

            match dialogue {
                Some(line) => {
                    let reply = TalkReply {
                        npc: npc_id.clone(),
                        dialogue: line,
                    };
                    let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
                    let _ = stream.write_all(msg.as_bytes());
                    log("INFO", "npc_talk", json!({"player": name, "npc": npc_id}));
                }
                None => send_err(stream, "404 NPC_NOT_FOUND"),
            }
        }
        Some("STATUS") => {
            let guard = players.lock().unwrap();
            if let Some(player) = guard.get(name) {
                let reply = StatusReply {
                    hp: player.hp,
                    max_hp: player.max_hp,
                    status: player.combat.clone(),
                };
                let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
                let _ = stream.write_all(msg.as_bytes());
            }
        }
        Some("QUEST") => {
            let Some(rest) = args.next() else {
                send_err(stream, "400 MALFORMED_COMMAND");
                return;
            };
            let npc_arg = rest.trim();
            let room = {
                let guard = players.lock().unwrap();
                guard.get(name).map(|p| p.room.clone())
            };
            let Some(room) = room else {
                return;
            };

            let npc_id = {
                let w = world.lock().unwrap();
                find_npc_in_room(&w, &room, npc_arg)
            };
            let Some(npc_id) = npc_id else {
                send_err(stream, "404 NPC_NOT_FOUND");
                return;
            };

            let offered: Vec<String> = {
                let w = world.lock().unwrap();
                w.npcs
                    .get(&npc_id)
                    .map(|n| n.quests.clone())
                    .unwrap_or_default()
            };
            if offered.is_empty() {
                send_err(stream, "406 NO_QUEST_AVAILABLE");
                return;
            }
            let mut guard = players.lock().unwrap();
            let w = world.lock().unwrap();
            let Some(player) = guard.get_mut(name) else {
                return;
            };
            for qid in offered.iter() {
                if player.quest_to_do.contains(qid) {
                    let total = w
                        .quests
                        .get(qid)
                        .and_then(|q| q.objective.count)
                        .unwrap_or(1);
                    let current = player.quest_progress.get(qid).copied().unwrap_or(0);
                    if current >= total {
                        player.quest_to_do.retain(|x| x != qid);
                        player.quest_dine.push(qid.clone());
                        if let Some(quest) = w.quests.get(qid) {
                            player.inventory.push(quest.reward.item.clone());
                        }
                        let reply = QuestReply {
                            quest_id: qid.clone(),
                            name: w
                                .quests
                                .get(qid)
                                .map(|q| q.name.clone())
                                .unwrap_or_else(|| qid.clone()),
                            description: w
                                .quests
                                .get(qid)
                                .map(|q| q.description.clone())
                                .unwrap_or_default(),
                            status: "completed".to_string(),
                            progress: "done".to_string(),
                        };
                        let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
                        let _ = stream.write_all(msg.as_bytes());
                        log(
                            "INFO",
                            "quest_completed",
                            json!({"player": name, "quest": qid}),
                        );
                        return;
                    } else {
                        let reply = QuestReply {
                            quest_id: qid.clone(),
                            name: w
                                .quests
                                .get(qid)
                                .map(|q| q.name.clone())
                                .unwrap_or_else(|| qid.clone()),
                            description: w
                                .quests
                                .get(qid)
                                .map(|q| q.description.clone())
                                .unwrap_or_default(),
                            status: "in_progress".to_string(),
                            progress: format!("{}/{}", current, total),
                        };
                        let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
                        let _ = stream.write_all(msg.as_bytes());
                        return;
                    }
                }
            }
            for qid in offered.iter() {
                if !player.quest_dine.contains(qid) {
                    player.quest_to_do.push(qid.clone());
                    player.quest_progress.insert(qid.clone(), 0);
                    let total = w
                        .quests
                        .get(qid)
                        .and_then(|q| q.objective.count)
                        .unwrap_or(1);
                    let reply = QuestReply {
                        quest_id: qid.clone(),
                        name: w
                            .quests
                            .get(qid)
                            .map(|q| q.name.clone())
                            .unwrap_or_else(|| qid.clone()),
                        description: w
                            .quests
                            .get(qid)
                            .map(|q| q.description.clone())
                            .unwrap_or_default(),
                        status: "in_progress".to_string(),
                        progress: format!("0/{}", total),
                    };
                    let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
                    let _ = stream.write_all(msg.as_bytes());
                    log(
                        "INFO",
                        "quest_accepted",
                        json!({"player": name, "quest": qid}),
                    );
                    return;
                }
            }

            send_err(stream, "412 QUEST_ALREADY_COMPLETED");
        }
        Some("QUESTS") => {
            let guard = players.lock().unwrap();
            let w = world.lock().unwrap();
            if let Some(player) = guard.get(name) {
                let mut list = Vec::new();
                for qid in &player.quest_to_do {
                    let total = w
                        .quests
                        .get(qid)
                        .and_then(|q| q.objective.count)
                        .unwrap_or(1);
                    let current = player.quest_progress.get(qid).copied().unwrap_or(0);
                    list.push(json!({
                        "quest_id": qid,
                        "name": w.quests.get(qid).map(|q| q.name.clone()).unwrap_or_else(|| qid.clone()),
                        "description": w.quests.get(qid).map(|q| q.description.clone()).unwrap_or_default(),
                        "status": "in_progress",
                        "progress": format!("{}/{}", current, total),
                    }));
                }
                for qid in &player.quest_dine {
                    list.push(json!({
                        "quest_id": qid,
                        "name": w.quests.get(qid).map(|q| q.name.clone()).unwrap_or_else(|| qid.clone()),
                        "description": w.quests.get(qid).map(|q| q.description.clone()).unwrap_or_default(),
                        "status": "completed",
                        "progress": "done",
                    }));
                }
                let msg = format!("OK {}\n", serde_json::to_string(&list).unwrap());
                let _ = stream.write_all(msg.as_bytes());
            }
        }
        Some("ATTACK") => {
            let already_fighting = {
                let guard = players.lock().unwrap();
                guard
                    .get(name)
                    .map(|p| p.combat == "in_combat")
                    .unwrap_or(false)
            };
            if !already_fighting {
                let Some(rest) = args.next() else {
                    send_err(stream, "400 MALFORMED_COMMAND");
                    return;
                };
                let npc_arg = rest.trim();
                let room = {
                    let guard = players.lock().unwrap();
                    guard.get(name).map(|p| p.room.clone())
                };
                let Some(room) = room else {
                    return;
                };

                let npc_id = {
                    let w = world.lock().unwrap();
                    find_npc_in_room(&w, &room, npc_arg)
                };
                let Some(npc_id) = npc_id else {
                    send_err(stream, "404 NPC_NOT_FOUND");
                    return;
                };
                let hostile = {
                    let w = world.lock().unwrap();
                    w.npcs.get(&npc_id).map(|n| n.hostile).unwrap_or(false)
                };
                if !hostile {
                    send_err(stream, "405 NPC_NOT_HOSTILE");
                    return;
                }
                let alive = {
                    let w = world.lock().unwrap();
                    w.npcs.get(&npc_id).map(|n| n.stats.hp > 0).unwrap_or(false)
                };
                if !alive {
                    send_err(stream, "404 NPC_NOT_FOUND");
                    return;
                }
                let engaged = {
                    let mut w = world.lock().unwrap();
                    match w.npcs.get_mut(&npc_id) {
                        Some(n) if n.engaged_by.is_none() => {
                            n.engaged_by = Some(name.to_string());
                            true
                        }
                        _ => false,
                    }
                };
                if !engaged {
                    send_err(stream, "407 NPC_BUSY");
                    return;
                }
                let (npc_hp, player_hp) = {
                    let mut guard = players.lock().unwrap();
                    let w = world.lock().unwrap();
                    if let Some(p) = guard.get_mut(name) {
                        p.combat = "in_combat".to_string();
                        p.target_npc = Some(npc_id.clone());
                    }
                    let npc_hp = w.npcs.get(&npc_id).map(|n| n.stats.hp).unwrap_or(0);
                    let player_hp = guard.get(name).map(|p| p.hp).unwrap_or(0);
                    (npc_hp, player_hp)
                };
                let reply = CombatReply {
                    action: "engage".to_string(),
                    attacker_hp: player_hp,
                    target_hp: npc_hp,
                    damage_dealt: 0,
                    damage_taken: 0,
                    status: "in_combat".to_string(),
                    message: Some(format!("combat started with {}", npc_id)),
                };
                let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
                let _ = stream.write_all(msg.as_bytes());

                {
                    let mut guard = players.lock().unwrap();
                    let evt = format!(
                        "EVT ROOM CHAT {} * dynamic combat engaged against {}\n",
                        name, npc_id
                    );
                    broadcast_room(&mut guard, &room, &evt, Some(name));
                }

                log(
                    "INFO",
                    "combat_started",
                    json!({"player": name, "npc": npc_id}),
                );
                return;
            }
            let npc_id = {
                let guard = players.lock().unwrap();
                guard.get(name).and_then(|p| p.target_npc.clone())
            };
            let Some(npc_id) = npc_id else {
                let mut guard = players.lock().unwrap();
                if let Some(p) = guard.get_mut(name) {
                    p.combat = "healthy".to_string();
                }
                send_err(stream, "407 NOT_IN_COMBAT");
                return;
            };

            let pv = {
                let guard = players.lock().unwrap();
                guard.get(name).map(|p| p.pv).unwrap_or(0)
            };

            let npc_already_dead = {
                let w = world.lock().unwrap();
                w.npcs.get(&npc_id).map(|n| n.stats.hp < 1).unwrap_or(true)
            };
            if npc_already_dead {
                {
                    let mut w = world.lock().unwrap();
                    if let Some(n) = w.npcs.get_mut(&npc_id) {
                        n.engaged_by = None;
                    }
                }
                {
                    let mut guard = players.lock().unwrap();
                    if let Some(p) = guard.get_mut(name) {
                        p.combat = "healthy".to_string();
                        p.target_npc = None;
                    }
                }
                send_err(stream, "404 NPC_NOT_FOUND");
                return;
            }
            let (npc_hp, npc_damage) = {
                let mut w = world.lock().unwrap();
                if let Some(n) = w.npcs.get_mut(&npc_id) {
                    n.stats.hp = n.stats.hp.saturating_sub(pv);
                    (n.stats.hp, n.stats.damage.unwrap_or(0))
                } else {
                    (0, 0)
                }
            };
            if npc_hp < 1 {
                let room = {
                    let guard = players.lock().unwrap();
                    guard.get(name).map(|p| p.room.clone()).unwrap_or_default()
                };
                let player_hp = {
                    let mut guard = players.lock().unwrap();
                    if let Some(p) = guard.get_mut(name) {
                        p.combat = "healthy".to_string();
                        p.target_npc = None;
                        p.hp
                    } else {
                        0
                    }
                };
                {
                    let mut w = world.lock().unwrap();
                    if let Some(location) = w.world.locations.get_mut(&room) {
                        location.npcs.retain(|id| id != &npc_id);
                    }
                    if let Some(n) = w.npcs.get_mut(&npc_id) {
                        n.engaged_by = None;
                    }
                }
                {
                    let mut guard = players.lock().unwrap();
                    let w = world.lock().unwrap();
                    if let Some(p) = guard.get_mut(name) {
                        apply_kill_quest_progress(p, &w, &npc_id);
                    }
                }
                let world_clone = Arc::clone(world);
                let npc_id_clone = npc_id.clone();
                let room_clone = room.clone();
                let respawn_delay = {
                    let w = world.lock().unwrap();
                    w.npcs
                        .get(&npc_id)
                        .and_then(|n| n.respawn_seconds)
                        .unwrap_or(30) as u64
                };
                thread::spawn(move || {
                    thread::sleep(Duration::from_secs(respawn_delay));
                    let mut w = world_clone.lock().unwrap();
                    if let Some(n) = w.npcs.get_mut(&npc_id_clone) {
                        n.stats.hp = n.stats.max_hp;
                        n.engaged_by = None;
                    }
                    if let Some(location) = w.world.locations.get_mut(&room_clone) {
                        if !location.npcs.contains(&npc_id_clone) {
                            location.npcs.push(npc_id_clone.clone());
                        }
                    }
                });
                let reply = CombatReply {
                    action: "attack".to_string(),
                    attacker_hp: player_hp,
                    target_hp: 0,
                    damage_dealt: pv,
                    damage_taken: 0,
                    status: "won".to_string(),
                    message: Some("you won the fight".to_string()),
                };
                let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
                let _ = stream.write_all(msg.as_bytes());

                {
                    let mut guard = players.lock().unwrap();
                    let evt = format!(
                        "EVT ROOM CHAT {} * dynamic combat defeated {}\n",
                        name, npc_id
                    );
                    broadcast_room(&mut guard, &room, &evt, Some(name));
                }

                log("INFO", "combat_won", json!({"player": name, "npc": npc_id}));
                return;
            }
            let player_hp = {
                let mut guard = players.lock().unwrap();
                if let Some(p) = guard.get_mut(name) {
                    p.hp = p.hp.saturating_sub(npc_damage);
                    p.hp
                } else {
                    0
                }
            };
            if player_hp < 1 {
                let (respawn_room, old_room) = {
                    let guard = players.lock().unwrap();
                    let old_r = guard.get(name).map(|p| p.room.clone()).unwrap_or_default();
                    let w = world.lock().unwrap();
                    (w.world.respawn_location.clone(), old_r)
                };
                {
                    let mut w = world.lock().unwrap();
                    if let Some(n) = w.npcs.get_mut(&npc_id) {
                        n.engaged_by = None;
                    }
                }
                {
                    let mut guard = players.lock().unwrap();
                    if let Some(p) = guard.get_mut(name) {
                        p.hp = 50;
                        p.combat = "healthy".to_string();
                        p.target_npc = None;
                        p.room = respawn_room.clone();
                    }
                }
                let reply = CombatReply {
                    action: "attack".to_string(),
                    attacker_hp: 0,
                    target_hp: npc_hp,
                    damage_dealt: pv,
                    damage_taken: npc_damage,
                    status: "dead".to_string(),
                    message: Some(format!(
                        "you died, respawned in {} with 50 hp",
                        respawn_room
                    )),
                };
                let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
                let _ = stream.write_all(msg.as_bytes());

                {
                    let mut guard = players.lock().unwrap();
                    let evt = format!(
                        "EVT ROOM CHAT {} * dynamic combat died fighting {}\n",
                        name, npc_id
                    );
                    broadcast_room(&mut guard, &old_room, &evt, Some(name));
                }

                log(
                    "INFO",
                    "player_died",
                    json!({"player": name, "npc": npc_id}),
                );
                return;
            }
            let reply = CombatReply {
                action: "attack".to_string(),
                attacker_hp: player_hp,
                target_hp: npc_hp,
                damage_dealt: pv,
                damage_taken: npc_damage,
                status: "in_combat".to_string(),
                message: None,
            };
            let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
            let _ = stream.write_all(msg.as_bytes());
            log(
                "INFO",
                "combat_round",
                json!({"player": name, "npc": npc_id}),
            );
        }
        Some("DEFEND") => {
            let npc_id = {
                let guard = players.lock().unwrap();
                guard.get(name).and_then(|p| {
                    if p.combat == "in_combat" {
                        p.target_npc.clone()
                    } else {
                        None
                    }
                })
            };
            let Some(npc_id) = npc_id else {
                send_err(stream, "407 NOT_IN_COMBAT");
                return;
            };

            let npc_damage = {
                let w = world.lock().unwrap();
                w.npcs
                    .get(&npc_id)
                    .and_then(|n| n.stats.damage)
                    .unwrap_or(0)
            };
            let reduced = npc_damage / 2;
            let player_hp = {
                let mut guard = players.lock().unwrap();
                if let Some(p) = guard.get_mut(name) {
                    p.hp = p.hp.saturating_sub(reduced);
                    p.hp
                } else {
                    0
                }
            };
            let npc_hp = {
                let w = world.lock().unwrap();
                w.npcs.get(&npc_id).map(|n| n.stats.hp).unwrap_or(0)
            };
            if player_hp < 1 {
                let (respawn_room, old_room) = {
                    let guard = players.lock().unwrap();
                    let old_r = guard.get(name).map(|p| p.room.clone()).unwrap_or_default();
                    let w = world.lock().unwrap();
                    (w.world.respawn_location.clone(), old_r)
                };
                {
                    let mut w = world.lock().unwrap();
                    if let Some(n) = w.npcs.get_mut(&npc_id) {
                        n.engaged_by = None;
                    }
                }
                {
                    let mut guard = players.lock().unwrap();
                    if let Some(p) = guard.get_mut(name) {
                        p.hp = 50;
                        p.combat = "healthy".to_string();
                        p.target_npc = None;
                        p.room = respawn_room.clone();
                    }
                }
                let reply = CombatReply {
                    action: "defend".to_string(),
                    attacker_hp: 0,
                    target_hp: npc_hp,
                    damage_dealt: 0,
                    damage_taken: reduced,
                    status: "dead".to_string(),
                    message: Some(format!(
                        "you died, respawned in {} with 50 hp",
                        respawn_room
                    )),
                };
                let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
                let _ = stream.write_all(msg.as_bytes());

                {
                    let mut guard = players.lock().unwrap();
                    let evt = format!(
                        "EVT ROOM CHAT {} * dynamic combat died fighting {}\n",
                        name, npc_id
                    );
                    broadcast_room(&mut guard, &old_room, &evt, Some(name));
                }

                log(
                    "INFO",
                    "player_died",
                    json!({"player": name, "npc": npc_id}),
                );
                return;
            }
            let reply = CombatReply {
                action: "defend".to_string(),
                attacker_hp: player_hp,
                target_hp: npc_hp,
                damage_dealt: 0,
                damage_taken: reduced,
                status: "in_combat".to_string(),
                message: None,
            };
            let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
            let _ = stream.write_all(msg.as_bytes());
            log(
                "INFO",
                "combat_defend",
                json!({"player": name, "npc": npc_id}),
            );
        }
        Some("FLEE") => {
            let (npc_id, room) = {
                let guard = players.lock().unwrap();
                let p = guard.get(name);
                let npc = p.and_then(|p| {
                    if p.combat == "in_combat" {
                        p.target_npc.clone()
                    } else {
                        None
                    }
                });
                let room = p.map(|p| p.room.clone()).unwrap_or_default();
                (npc, room)
            };
            let Some(npc_id) = npc_id else {
                send_err(stream, "407 NOT_IN_COMBAT");
                return;
            };
            {
                let mut w = world.lock().unwrap();
                if let Some(n) = w.npcs.get_mut(&npc_id) {
                    n.engaged_by = None;
                }
            }
            let player_hp = {
                let mut guard = players.lock().unwrap();
                if let Some(p) = guard.get_mut(name) {
                    p.combat = "healthy".to_string();
                    p.target_npc = None;
                    p.hp
                } else {
                    0
                }
            };
            let reply = CombatReply {
                action: "flee".to_string(),
                attacker_hp: player_hp,
                target_hp: 0,
                damage_dealt: 0,
                damage_taken: 0,
                status: "fled".to_string(),
                message: Some("you fled the combat".to_string()),
            };
            let msg = format!("OK {}\n", serde_json::to_string(&reply).unwrap());
            let _ = stream.write_all(msg.as_bytes());

            {
                let mut guard = players.lock().unwrap();
                let evt = format!(
                    "EVT ROOM CHAT {} * dynamic combat fled from {}\n",
                    name, npc_id
                );
                broadcast_room(&mut guard, &room, &evt, Some(name));
            }

            log(
                "INFO",
                "combat_flee",
                json!({"player": name, "npc": npc_id}),
            );
        }
        Some(cmd_inconnue) => {
            send_err(stream, "400 MALFORMED_COMMAND");
            log(
                "WARN",
                "unknown_command",
                json!({"player": name, "command": cmd_inconnue}),
            );
        }
        None => {
            send_err(stream, "400 MALFORMED_COMMAND");
        }
    }
}
