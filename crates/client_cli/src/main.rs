use std::env;
use std::io::{stdin, BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 || args[1].parse::<SocketAddr>().is_err() {
        eprintln!("Bad arguments, usage: client_cli <ip:port>, no localhost is allowed.");
        std::process::exit(1);
    }
    let addr = args[1].clone();

    let stream = loop {
        match TcpStream::connect(&addr) {
            Ok(s) => break s,
            Err(e) => {
                eprintln!("Failed: {e}, retrying...");
                thread::sleep(Duration::from_secs(1));
            }
        }
    };
    println!("Connected to {addr}");

    let read_stream = stream.try_clone().expect("Error cloning stream");
    let reader_handle = thread::spawn(move || {
        let reader = BufReader::new(read_stream);
        for line in reader.lines() {
            match line {
                Ok(content) => println!("<< {content}"),
                Err(_) => break,
            }
        }
        println!("(connection closed)");
    });

    let mut write_stream = stream;
    for line in stdin().lock().lines() {
        let Ok(line) = line else { break };
        let cmd = line.trim();
        if cmd.is_empty() {
            continue;
        }
        println!(">> {cmd}");
        if write_stream.write_all(format!("{cmd}\n").as_bytes()).is_err() {
            eprintln!("(connection lost)");
            break;
        }
        if cmd == "QUIT" {
            break;
        }
    }
    let _ = reader_handle.join();
}