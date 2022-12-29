use std::time::Duration;

use relay_man::server::RelayServer;

fn main() {
    server();
}

fn server() {
    let mut server = RelayServer::new(Duration::from_secs(5), Duration::from_secs(10)).unwrap();
    loop {
        server.step()
    }
}
