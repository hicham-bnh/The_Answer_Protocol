use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use std::net::TcpStream;
use std::io::Write;


pub fn connect_user(line: &str, stream: &mut TcpStream, players: &Arc<Mutex<HashSet<String>>>) -> bool{
    let mut args = line.splitn(2, ' ');
    let mut name = String::new();
    let mut guard = players.lock().unwrap();
    match args.next(){
        Some("CONNECT") => {
            if let Some(arg_name) = args.next() {
                name.push_str(arg_name);
                if guard.contains(&name) {
                    stream.write_all(b"ERR 201 NAME_IN_USE\n").expect("write failed");
                    return false;
                }
                else {
                    guard.insert(name.to_string());
                    stream.write_all(b"OK connected\n").expect("write failed");
                    return true;
                }
            }
            else {
                stream.write_all(b"please connect with your username\n").expect("Failder to write reponse");
                return false;
            }
        }
        Some("QUIT") => {
            stream.write_all(b"OK bye\n").expect("Failder to write reponse");
            println!("USER QUIT");
            return false;
        }
        Some(cmd_inconnue) => {
            stream.write_all(b"please connect\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR: {}", cmd_inconnue);
            return false;
        }
        None => {
            stream.write_all(b"ERROR COMMANDE\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR");
            return false;
        }
    }
}



pub fn parse_command(line: &str, stream: &mut TcpStream) {
    let mut args = line.splitn(2, ' ');
    match args.next(){
        Some("CONNECT") => {
             stream.write_all(b"you are connect yet\n").expect("Failder to write reponse");
        }
        Some("LOOK") => {
            stream.write_all(b"OK {room: example}\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("MOVE") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE MOVE");
        }
        Some("QUIT") => {
            stream.write_all(b"OK bye\n").expect("Failder to write reponse");
            println!("USER QUIT");
        }
        Some("CHAT") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
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
        None => {
            stream.write_all(b"ERROR COMMANDE\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR");
        }
        Some(cmd_inconnue) => {
            stream.write_all(b"ERROR COMMANDE\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR: {}", cmd_inconnue);
        }
    }
}
