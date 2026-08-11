use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::net::TcpStream;
use std::io::Write;
use crate::Player;

use crate::world_struct::world_struct::GameWorld;

pub fn connect_user(
    line: &str,
    stream: &mut TcpStream,
    players: &Arc<Mutex<HashMap<String, Player>>>, 
    spawn_room: String
    ) -> (bool, String){
    let mut args = line.splitn(2, ' ');
    let mut name = String::new();
    let mut guard = players.lock().unwrap();
    match args.next(){
        Some("CONNECT") => {
            if let Some(arg_name) = args.next() {
                name.push_str(arg_name);
                if guard.contains_key(&name) {
                    stream.write_all(b"ERR 201 NAME_IN_USE\n").expect("write failed");
                    return (false, String::new());
                }
                else {
                    let stream_for_map = stream.try_clone().expect("clone failed");
                    let player = Player {
                        stream: Mutex::new(stream_for_map),
                        room: spawn_room.to_string(),
                        name: name.to_string()
                    };
                    guard.insert(name.clone(), player);
                    stream.write_all(b"OK connected\n").expect("write failed");
                    return (true, name);
                }
            }
            else {
                stream.write_all(b"please connect with your username\n").expect("Failder to write reponse");
                return (false, String::new());
            }
        }
        Some("QUIT") => {
            stream.write_all(b"OK bye\n").expect("Failder to write reponse");
            println!("USER QUIT");
            return (false, String::new());
        }
        Some(cmd_inconnue) => {
            stream.write_all(b"please connect\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR: {}", cmd_inconnue);
            return (false, String::new());
        }
        _none => {
            stream.write_all(b"ERROR COMMANDE\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR");
            return (false, String::new());
        }
    }
}



pub fn parse_command(
    line: &str,
    stream: &mut TcpStream,
    players: &Arc<Mutex<HashMap<String, Player>>>,
    name: &str,
    world: &GameWorld
    ) {
    let mut args = line.splitn(2, ' ');
    match args.next(){
        Some("CONNECT") => {
            stream.write_all(b"you are connect yet\n").expect("Failder to write reponse");
        }
        Some("LOOK") => {
            let mut guard = players.lock().unwrap();
            let mut room_str = String::new();
            let mut players: Vec<String> = Vec::new();
            if let Some(player) = guard.get(name) {
                room_str = player.room.clone();
            }
            for (_client_name, client_steam) in guard.iter_mut(){
                 if client_steam.room.to_string() == room_str {
                     players.push(_client_name.to_string());
                 }
             }
             if let Some(player) = guard.get(name) {
                let items = &world.world.locations.get(&room_str).unwrap().items;
                let npcs = &world.world.locations.get(&room_str).unwrap().npcs;
                let msg = format!(
                    "OK {{ \"room\": {}, \"players\": {:?}, \"items\": {:?}, , \"npcs\": {:?} }}\n",
                    room_str, players, items, npcs
                );
                let mut guard_stream = player.stream.lock().unwrap();
                guard_stream.write_all(msg.as_bytes()).expect("Failder to write reponse");
            }
            println!("USER USE LOOK");
        }
        Some("MOVE") => {
            let mut chat_args = args.next().expect("REASON").splitn(2, ' ');
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE MOVE");
        }
        Some("QUIT") => {
            println!("{} QUIT", name);
            let mut guard = players.lock().unwrap();
            guard.remove(name);
            stream.write_all(b"OK bye\n").expect("Failder to write reponse");
        }
        Some("CHAT") => {
            let mut chat_args = args.next().expect("REASON").splitn(2, ' ');
            if chat_args.next() == Some("GLOBAL"){
                //let target = sub_args.next().unwrap_or("");
                let chat_msg = chat_args.next().unwrap_or("");
                let mut guard = players.lock().unwrap();
                stream.write_all(b"OK\n").expect("Failder to write reponse");
                for (_client_name, client_steam) in guard.iter_mut()
                {
                    let msg = format!("EVT GLOBAL CHAT {} {}\n", name, chat_msg);
                    client_steam.stream.lock().unwrap().write_all(msg.as_bytes()).expect("write failed");
                }
            }
            println!("{} USE CHAT", name);
        }
        Some("WHO") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("GROUP") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("TAKE") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("DROP") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("INVENTORY") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("TALK") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("ATTACK") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("STATUS") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("QUEST") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("QUESTS") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some(cmd_inconnue) => {
            stream.write_all(b"ERROR COMMANDE\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR: {}", cmd_inconnue);
        }
        _none => {
            stream.write_all(b"ERROR COMMANDE\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR");
        }
    }
}
