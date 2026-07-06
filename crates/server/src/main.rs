use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use std::io::{Write, BufReader, BufRead};
use std::net::{TcpListener, TcpStream};

mod protocol {
    pub mod command;
}
use protocol::command::parse_command;
use protocol::command::connect_user;


fn lunch(mut stream: TcpStream, players: Arc<Mutex<HashSet<String>>>){
    let mut is_connect = false;
    let stream_clone = stream.try_clone().expect("clone");
    let read_buf = BufReader::new(stream_clone);
    stream.write_all(b"OK hello proto=1\n").expect("Failder to write reponse");
    for line in read_buf.lines(){
        let line = line.expect("erreur de lecture");
        println!("client try: {}", line);
        if !is_connect{
            is_connect = connect_user(&line, &mut stream, &players);
            println!("{:?}", is_connect);
            continue;
        }
        parse_command(&line, &mut stream);
        //let reponse = "hello, client".as_bytes();
    }
}

fn main(){
    let listener = TcpListener::bind("127.0.0.1:8080").expect("failde to bind");
    println!("Server run");
    let players: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    for stream in listener.incoming(){
        match stream{
            Ok(stream) => {
                let players_clone = Arc::clone(&players);
                std::thread::spawn(|| lunch(stream, players_clone));
            }
            Err(e) => {
                eprintln!("Failde to conection: {}", e);
            }
        }
    }
}