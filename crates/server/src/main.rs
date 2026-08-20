use serde_json::json;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Instant;

mod protocol {
    pub mod command;
}
pub mod world_struct;

pub struct Player {
    pub stream: Mutex<TcpStream>,
    pub room: String,
    pub name: String,
    pub inventory: Vec<String>,
    pub hp: u32,
    pub max_hp: u32,
    pub pv: u32,
    pub combat: String,
    pub quest_to_do: Vec<String>,
    pub quest_dine: Vec<String>,
    pub quest_progress: HashMap<String, u32>,
    pub target_npc: Option<String>,
    pub group: Option<String>,
    pub invite_from: Option<String>,
    pub dialogue_progress: HashMap<String, usize>,
    pub command: u32,
    pub time_command: Instant,
}

use protocol::command::{connect_user, disconnect_player, log, parse_command};
use world_struct::GameWorld;

fn parse_world() -> GameWorld {
    let content = match fs::read_to_string("config/world.yaml") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read config/world.yaml: {e}");
            std::process::exit(1);
        }
    };
    let mut game_world: GameWorld = match serde_yaml::from_str(&content) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Invalid world.yaml: {e}");
            std::process::exit(1);
        }
    };
    for npc in game_world.npcs.values_mut() {
        if npc.stats.max_hp == 0 {
            npc.stats.max_hp = npc.stats.hp;
        }
    }
    validate_world(&game_world);
    game_world
}

fn validate_world(w: &GameWorld) {
    let mut errors: Vec<String> = Vec::new();
    if !w.world.locations.contains_key(&w.world.start_location) {
        errors.push(format!(
            "start_location '{}' does not exist",
            w.world.start_location
        ));
    }
    if !w.world.locations.contains_key(&w.world.respawn_location) {
        errors.push(format!(
            "respawn_location '{}' does not exist",
            w.world.respawn_location
        ));
    }
    for (id, loc) in &w.world.locations {
        for (dir, dest) in &loc.exits {
            if !w.world.locations.contains_key(dest) {
                errors.push(format!(
                    "exit '{dir}' of '{id}' points to unknown room '{dest}'"
                ));
            }
        }
        for item in &loc.items {
            if !w.items.contains_key(item) {
                errors.push(format!("room '{id}' contains unknown item '{item}'"));
            }
        }
        for npc in &loc.npcs {
            if !w.npcs.contains_key(npc) {
                errors.push(format!("room '{id}' contains unknown npc '{npc}'"));
            }
        }
    }
    for (id, npc) in &w.npcs {
        for quest in &npc.quests {
            if !w.quests.contains_key(quest) {
                errors.push(format!("npc '{id}' offers unknown quest '{quest}'"));
            }
        }
    }
    for (id, quest) in &w.quests {
        if !w.npcs.contains_key(&quest.giver) {
            errors.push(format!(
                "quest '{id}' giver '{}' does not exist",
                quest.giver
            ));
        }
        if let Some(item) = &quest.objective.item {
            if !w.items.contains_key(item) {
                errors.push(format!(
                    "quest '{id}' objective item '{item}' does not exist"
                ));
            }
        }
        if let Some(target) = &quest.objective.target {
            if !w.npcs.contains_key(target) {
                errors.push(format!(
                    "quest '{id}' objective target '{target}' does not exist"
                ));
            }
        }
        if let Some(npc) = &quest.objective.deliver_to {
            if !w.npcs.contains_key(npc) {
                errors.push(format!("quest '{id}' deliver_to '{npc}' does not exist"));
            }
        }
        if !w.items.contains_key(&quest.reward.item) {
            errors.push(format!(
                "quest '{id}' reward item '{}' does not exist",
                quest.reward.item
            ));
        }
    }
    let mut names: HashMap<&str, Vec<&String>> = HashMap::new();
    for (id, item) in &w.items {
        names.entry(item.name.as_str()).or_default().push(id);
    }
    for (name, ids) in &names {
        if ids.len() > 1 {
            errors.push(format!(
                "display name '{name}' is shared by several items: {ids:?}"
            ));
        }
    }
    let mut names: HashMap<&str, Vec<&String>> = HashMap::new();
    for (id, npc) in &w.npcs {
        names.entry(npc.name.as_str()).or_default().push(id);
    }
    for (name, ids) in &names {
        if ids.len() > 1 {
            errors.push(format!(
                "display name '{name}' is shared by several npcs: {ids:?}"
            ));
        }
    }

    if w.world.locations.contains_key(&w.world.start_location) {
        let mut seen: HashSet<&String> = HashSet::new();
        let mut queue: VecDeque<&String> = VecDeque::new();
        queue.push_back(&w.world.start_location);
        while let Some(id) = queue.pop_front() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(loc) = w.world.locations.get(id) {
                for dest in loc.exits.values() {
                    if w.world.locations.contains_key(dest) {
                        queue.push_back(dest);
                    }
                }
            }
        }
        for id in w.world.locations.keys() {
            if !seen.contains(id) {
                errors.push(format!(
                    "room '{id}' cannot be reached from '{}'",
                    w.world.start_location
                ));
            }
        }
    }

    let mut edges: HashSet<(&String, &String)> = HashSet::new();
    for (id, loc) in &w.world.locations {
        for dest in loc.exits.values() {
            if id != dest && w.world.locations.contains_key(dest) {
                let pair = if id < dest { (id, dest) } else { (dest, id) };
                edges.insert(pair);
            }
        }
    }
    if !w.world.locations.is_empty() && edges.len() < w.world.locations.len() {
        errors.push(
            "world map has no loop: a full circuit must be possible (line-only maps are not allowed)"
                .to_string(),
        );
    }

    if !errors.is_empty() {
        eprintln!("world.yaml validation failed:");
        for e in &errors {
            eprintln!("  - {e}");
        }
        std::process::exit(1);
    }
}

fn lunch(
    mut stream: TcpStream,
    players: Arc<Mutex<HashMap<String, Player>>>,
    spawn_room: String,
    world: Arc<Mutex<GameWorld>>,
    next_group_id: Arc<Mutex<u32>>,
) {
    let mut is_connect = false;
    let mut name = String::new();

    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let stream_clone = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            log(
                "ERROR",
                "stream_clone_failed",
                json!({"ip": peer, "error": e.to_string()}),
            );
            return;
        }
    };
    let read_buf = BufReader::new(stream_clone);
    let _ = stream.write_all(b"OK hello proto=1\n");
    log("INFO", "connection_opened", json!({"ip": peer}));
    for line in read_buf.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                log(
                    "WARN",
                    "read_error",
                    json!({"ip": peer, "player": name, "error": e.to_string()}),
                );
                break;
            }
        };
        log(
            "INFO",
            "command_received",
            json!({"player": name, "line": line}),
        );
        if line.trim() == "QUIT" {
            if is_connect {
                parse_command(&line, &mut stream, &players, &name, &world, &next_group_id);
            } else {
                let _ = connect_user(&line, &mut stream, &players, spawn_room.clone());
            }
            break;
        }
        if !is_connect {
            (is_connect, name) = connect_user(&line, &mut stream, &players, spawn_room.clone());
            continue;
        }
        parse_command(&line, &mut stream, &players, &name, &world, &next_group_id);
    }
    if is_connect {
        disconnect_player(&name, &players, &world);
    }
    log(
        "INFO",
        "connection_closed",
        json!({"ip": peer, "player": name}),
    );
}

fn main() {
    let listener = match TcpListener::bind("127.0.0.1:8080") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Cannot bind 127.0.0.1:8080: {e}");
            std::process::exit(1);
        }
    };
    let game_world = Arc::new(Mutex::new(parse_world()));
    let spawn_room = game_world.lock().unwrap().world.start_location.clone();
    println!("Server run");
    let players: Arc<Mutex<HashMap<String, Player>>> = Arc::new(Mutex::new(HashMap::new()));
    let next_group_id: Arc<Mutex<u32>> = Arc::new(Mutex::new(1));
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let players_clone = Arc::clone(&players);
                let world_clone = Arc::clone(&game_world);
                let next_group_id_clone = Arc::clone(&next_group_id);
                let value = spawn_room.clone();
                std::thread::spawn(move || {
                    lunch(
                        stream,
                        players_clone,
                        value.clone(),
                        world_clone,
                        next_group_id_clone,
                    )
                });
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {e}");
            }
        }
    }
}
