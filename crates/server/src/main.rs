use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::io::{Write, BufReader, BufRead};
use std::net::{TcpListener, TcpStream};
use std::fs;


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
    pub pv: u32,
    pub combat: String
}
use protocol::command::parse_command;
use protocol::command::connect_user;
use world_struct::world_struct::GameWorld;


fn parse_world() -> GameWorld {
    let world = fs::read_to_string("config/world.yaml") .expect("Impossible de lire world.yaml");
    let game_world: GameWorld = serde_yaml::from_str(&world).expect("ERROR SERDE");
    game_world
}

fn lunch(mut stream: TcpStream, players: Arc<Mutex<HashMap<String, Player>>>, spawn_room: String, world: Arc<Mutex<GameWorld>>){
    let mut is_connect = false;
    let mut name = String::new();
    let stream_clone = stream.try_clone().expect("clone");
    let read_buf = BufReader::new(stream_clone);
    stream.write_all(b"OK hello proto=1\n").expect("Failder to write reponse");
    for line in read_buf.lines(){
        let line = line.expect("erreur de lecture");
        println!("client try: {}", line);
        if !is_connect{
            (is_connect, name) = connect_user(&line, &mut stream, &players, spawn_room.clone());
            continue;
        }
        parse_command(&line, &mut stream, &players, &name, &world);
    }
}

fn main(){
    let listener = TcpListener::bind("127.0.0.1:8080").expect("failde to bind");
    let game_world = Arc::new(Mutex::new(parse_world()));
    let spawn_room = game_world.lock().unwrap().world.start_location.clone();
    println!("Server run");
    let players: Arc<Mutex<HashMap<String, Player>>> = Arc::new(Mutex::new(HashMap::new()));
    for stream in listener.incoming(){
        match stream{
            Ok(stream) => {
                let players_clone = Arc::clone(&players);
                let world_clone =  Arc::clone(&game_world);
                let value = spawn_room.clone();
                std::thread::spawn(move || lunch(stream, players_clone, value.clone(), world_clone));
            }
            Err(e) => {
                eprintln!("Failde to conection: {}", e);
            }
        }
    }
}
