use std::net::TcpStream;
use std::io::Write;
use std::io::{stdin, Read};

// fn client_connection(mut stream: TcpStream) {
//     let mut buf = String::new();
// }


fn main() {
    println!("Tentative de connexion au serveur...");
    match TcpStream::connect("127.0.0.1:8080") {
        Ok(mut stream) => {
            let mut stream_lecture = stream.try_clone().expect("clone failed");
            let handle = std::thread::spawn(move || {
                loop {
                    let mut buf_reception = [0u8; 512];
                    let n = match stream_lecture.read(&mut buf_reception) {
                        Ok(0) => {
                            println!("Server deconnected");
                            break;
                        }
                        Ok(n) => n,
                        Err(e) => {
                            println!("erreur reading : {e}");
                            break;
                        }
                    };
                    let texte = String::from_utf8_lossy(&buf_reception[..n]);
                    print!("s: {}", texte);
                    if texte == "OK bye\n"{
                        break;
                    }
                }
            });
            let mut quit = false;
            while !quit {
                let mut buf = String::new();
                if let Err(e) = stdin().read_line(&mut buf) {
                    println!("erreur : {e}");
                }
                if let Err(e) = stream.write(buf.as_bytes()) {
                    println!("erreur : {e}");
                }
                if buf.trim() == "QUIT" {
                    quit = true;
                }
            }
            let _ = handle.join();
        }
        Err(e) => {
            println!("errer conection to the server : {}", e);
        }
    }
}