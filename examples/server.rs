use std::time::Duration;

use relay::server::RelayServer;

fn main() {
    let mut server = RelayServer::new().unwrap();
    println!("Server Created");
    loop {
        std::thread::sleep(Duration::from_millis(50));
        server.step();
        println!("");
        for client in server.clients.iter() {
            println!("Client: {}", client.name);
        }
        println!("");
    }
}
