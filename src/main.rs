use std::time::Duration;

use relay_man::server::RelayServer;

fn main() {
    server();
}

fn server() {
    let mut server = RelayServer::new().unwrap();
    loop {
        std::thread::sleep(Duration::from_millis(50));
        server.step()
    }
}
