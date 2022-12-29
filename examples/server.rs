use std::time::Duration;

use relay_man::server::RelayServer;

fn main() {
    let mut server = RelayServer::new().unwrap();
    println!("Server Created");

    let mut lasts = Vec::new();

    loop {
        std::thread::sleep(Duration::from_millis(0));
        server.step();
        for client in server.clients.iter() {
            if !lasts.contains(&(
                client.session,
                client.ports.clone(),
                client.to_connect.clone(),
            )) {
                println!(
                    "Client: {}, session: {} adress: {:?}, ports: {:?}, to_connect: {:?}",
                    client.name, client.session, client.adress, client.ports, client.to_connect
                );
                lasts.retain(|last| last.0 != client.session);
                lasts.push((
                    client.session,
                    client.ports.clone(),
                    client.to_connect.clone(),
                ))
            }
        }
    }
}
