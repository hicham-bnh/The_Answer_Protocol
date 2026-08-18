use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

mod protocol {
    pub mod command;
}
mod world_struct {
    pub mod world_struct;
}

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
}

use protocol::command::{connect_user, disconnect_player, log, parse_command};
use world_struct::world_struct::GameWorld;

fn parse_world() -> GameWorld {
    let world = fs::read_to_string("config/world.yaml").expect("Impossible de lire world.yaml");
    let mut game_world: GameWorld = serde_yaml::from_str(&world).expect("ERROR SERDE");
    for npc in game_world.npcs.values_mut() {
        if npc.stats.max_hp == 0 {
            npc.stats.max_hp = npc.stats.hp;
        }
    }
    game_world
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
        disconnect_player(&name, &players);
    }
    log(
        "INFO",
        "connection_closed",
        json!({"ip": peer, "player": name}),
    );
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").expect("failde to bind");
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
                eprintln!("Failde to conection: {}", e);
            }
        }
    }
}
