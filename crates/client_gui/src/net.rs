use eframe::egui;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::io::Write;

pub fn start(address: String, tx: Sender<String>, rx_out: Receiver<String>, ctx: egui::Context) {
    thread::spawn(move || match TcpStream::connect(address) {
        Ok(stream) => {
            println!("Connected !");
            
            let mut stream_out = stream.try_clone().expect("Error creating output stream");

            thread::spawn(move || {
                while let Ok(content_out) = rx_out.recv() {
                    let msg = format!("{content_out}\n");
                    if stream_out.write_all(msg.as_bytes()).is_err() {
                        break;
                    }
                }
            });
            
            let reader = BufReader::new(stream);
            for line in reader.lines() {
                if let Ok(content) = line {
                    match tx.send(content) {
                        Ok(_) => ctx.request_repaint(),
                        Err(_) => break,
                    };
                } else {
                    println!("Connexion died unexpectedly !");
                    break;
                };
            }
        }
        Err(e) => println!("Failed: {} !", e),
    });
}
