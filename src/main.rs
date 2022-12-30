use std::time::Duration;

use relay_man::server::RelayServer;

fn main() {
    server();
}

fn server() {
    let mut server = RelayServer::new("0.0.0.0", Duration::from_secs(5)).unwrap();
    loop {
        server.step()
    }
}
