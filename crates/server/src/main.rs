use std::io::{Write, BufReader, BufRead};
use std::net::{TcpListener, TcpStream};
mod protocol {
    pub mod command;
}
use protocol::command::parse_command;


fn connect_client(mut stream: TcpStream){
    let stream_clone = stream.try_clone().expect("clone");
    let read_buf = BufReader::new(stream_clone);
    stream.write_all(b"OK hello proto=1\n").expect("Failder to write reponse");
    for line in read_buf.lines(){
        let line = line.expect("erreur de lecture");
        println!("{}", line);
        parse_command(&line, &mut stream);
        //let reponse = "hello, client".as_bytes();
    }
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