use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::thread;

fn main() {
    println!("Tentative de connexion au serveur...");

    let stream = match TcpStream::connect("127.0.0.1:8080") {
        Ok(s) => {
            println!("Connexion réussie !");
            s
        }
        Err(e) => {
            println!("Échec de connexion : {}", e);
            return;
        }
    };

    let mut write_stream = stream.try_clone().expect("clone failed");
    let read_stream = stream.try_clone().expect("clone failed");

    // Thread dédié : lit en continu tout ce qui vient du serveur, et l'affiche
    thread::spawn(move || {
        let reader = BufReader::new(read_stream);
        for line in reader.lines() {
            match line {
                Ok(l) => println!("S: {}", l),
                Err(_) => {
                    println!("Serveur déconnecté.");
                    break;
                }
            }
        }
    });

    // Thread principal : lit le clavier et envoie au serveur, sans attendre de réponse
    let stdin = std::io::stdin();
    loop {
        let mut input = String::new();
        stdin.read_line(&mut input).expect("erreur stdin");
        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        write_stream.write_all(format!("{}\n", input).as_bytes()).expect("erreur d'écriture");
        if input == "QUIT" {
            break;
        }
    }
}