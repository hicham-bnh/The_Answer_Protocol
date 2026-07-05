use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn main() {
    println!("Tentative de connexion au serveur...");

    let mut stream = match TcpStream::connect("127.0.0.1:8080") {
        Ok(s) => {
            println!("Connexion réussie !");
            s
        }
        Err(e) => {
            println!("Échec de connexion : {}", e);
            return;
        }
    };

    let stream_clone = stream.try_clone().expect("clone failed");
    let mut reader = BufReader::new(stream_clone);

    // Lire la greeting envoyée par le serveur
    let mut greeting = String::new();
    reader.read_line(&mut greeting).expect("erreur de lecture greeting");
    print!("Serveur dit: {}", greeting);

    // Envoyer une commande CONNECT
    let cmd = "LOOK alice\n";
    stream.write_all(cmd.as_bytes()).expect("erreur d'écriture");
    println!("Envoyé: {}", cmd.trim());

    // Lire la réponse à CONNECT
    let mut response = String::new();
    reader.read_line(&mut response).expect("erreur de lecture réponse");
    print!("Serveur dit: {}", response);

    // Boucle interactive : tape des commandes au clavier
    let stdin = std::io::stdin();
    loop {
        let mut input = String::new();
        stdin.read_line(&mut input).expect("erreur stdin");
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        stream.write_all(format!("{}\n", input).as_bytes()).expect("erreur d'écriture");

        if input.eq_ignore_ascii_case("QUIT") {
            break;
        }

        let mut response = String::new();
        if reader.read_line(&mut response).unwrap_or(0) == 0 {
            println!("Serveur déconnecté.");
            break;
        }
        print!("Serveur dit: {}", response);
    }
}