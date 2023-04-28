use std::time::Duration;

use relay_man::server::RelayServer;

fn main() {
    env_logger::init();
    let mut server = RelayServer::new("0.0.0.0", Duration::from_secs(5)).unwrap();
    println!("Server Started");

    loop {
        server.step();
    }
}
