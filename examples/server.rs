use std::time::Duration;

use relay_man::server::{ClientStage, RelayServer};

fn main() {
    let mut server = RelayServer::new(Duration::from_secs(5), Duration::from_secs(10)).unwrap();
    println!("Server Created");

    let mut lasts = Vec::new();

    loop {
        std::thread::sleep(Duration::from_millis(0));
        server.step();
        for client in server.clients.iter() {
            if let ClientStage::Registered(rclient) = &client.stage {
                if !lasts.contains(&(
                    client.session,
                    rclient.ports.clone(),
                    rclient.to_connect.clone(),
                )) {
                    println!(
                        "Client: {}, session: {} adress: {:?}, ports: {:?}, to_connect: {:?}",
                        rclient.name,
                        client.session,
                        rclient.adress,
                        rclient.ports,
                        rclient.to_connect
                    );
                    lasts.retain(|last| last.0 != client.session);
                    lasts.push((
                        client.session,
                        rclient.ports.clone(),
                        rclient.to_connect.clone(),
                    ))
                }
            }
        }
    }
}
