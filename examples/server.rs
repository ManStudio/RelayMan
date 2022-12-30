use std::time::Duration;

use relay_man::server::{ClientStage, RelayServer};

fn main() {
    let mut server = RelayServer::new(Duration::from_secs(5)).unwrap();
    println!("Server Created");

    loop {
        std::thread::sleep(Duration::from_millis(0));
        server.step();
        println!("L:");
        for client in server.clients.iter() {
            if let ClientStage::Registered(rclient) = &client.stage {
                println!(
                    "    Client: {}, session: {} adress: {:?}, ports: {:?}, to_connect: {:?}",
                    rclient.name, client.session, rclient.adress, rclient.ports, rclient.to_connect
                );
            }
        }
    }
}
