struct TapClient {
    server_adress: String
}

fn main(){
    let client = TapClient{
        server_adress: String::from("127.0.0.1:8080")
    };
    println!("Adresse du serveur : {}", client.server_adress);
}