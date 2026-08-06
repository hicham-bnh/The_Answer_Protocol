use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::sync::mpsc::Sender;
use std::thread;

pub fn start(address: String, tx: Sender<String>) {
    thread::spawn(move || match TcpStream::connect(address) {
        Ok(stream) => {
            println!("Connected !");
            let reader = BufReader::new(stream);
            for line in reader.lines() {
                if let Ok(content) = line {
                    println!("{}", content);
                } else {
                    println!("Connexion died unexpectedly !");
                    break;
                };
            }
        }
        Err(e) => println!("Failed: {} !", e),
    });
}
