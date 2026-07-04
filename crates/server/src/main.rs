use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn connect_client(mut stream: TcpStream){
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).expect("Fail de lecture");
    let request = String::from_utf8_lossy(&buffer[..]);
    println!("ca marche: {}", request);
    let reponse = "hello, client".as_bytes();
    stream.write(reponse).expect("Failder to write reponse");
}

fn main(){
    let listener = TcpListener::bind("127.0.0.1:8080").expect("failde to bind");
    println!("Server run");
    for stream in listener.incoming(){
        match stream{
            Ok(stream) => {
                std::thread::spawn(|| connect_client(stream));
            }
            Err(e) => {
                eprintln!("Failde to conection: {}", e);
            }
        }
    }
}