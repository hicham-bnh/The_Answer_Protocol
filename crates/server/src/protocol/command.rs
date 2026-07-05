use std::net::TcpStream;
use std::io::Write;


pub fn parse_command(line: &str, stream: &mut TcpStream) {
        let mut args = line.splitn(2, ' ');
        match args.next(){
            Some("CONNECT") => {
                 stream.write_all(b"OK connected\n").expect("Failder to write reponse");
                println!("COMANDE CONNECT MARCHE");
            }
            Some("LOOK") => {
                stream.write_all(b"OK LOOK\n").expect("Failder to write reponse");
                println!("COMANDE LOOK MARCHE");
            }
            None => {
                println!("Ligne vide reçu");
            }
            Some(cmd_inconnue) => {
            println!("Commande inconnue reçue : {}", cmd_inconnue);
            }
        }
}