use std::time::Duration;

use relay_man::server::RelayServer;

fn main() {
    let mut server = RelayServer::new().unwrap();
    println!("Server Created");
    loop {
        std::thread::sleep(Duration::from_millis(0));
        server.step();
        for client in server.clients.iter() {
            println!(
                "Client: {}, session: {} adress: {:?}, ports: {:?}, to_connect: {:?}",
                client.name, client.session, client.adress, client.ports, client.to_connect
            );
        }
    }
}
