use std::{mem::MaybeUninit, net::ToSocketAddrs, thread::JoinHandle, time::Duration};

use rand::{random, Rng};
use relay_man::{
    client::{response::Conn, ConnectionInfo, RelayClient},
    common::{
        adress::Adress,
        packets::{Search, SearchType},
    },
};
use socket2::{Domain, SockAddr, Socket};

fn main() {
    println!("Starting client");
    let info = ConnectionInfo {
        client: "Test".into(),
        name: "konkito".into(),
        public: vec![random(), random(), random(), random()],
        other: vec![],
        privacy: false,
    };
    println!("Info: {:?}", info);
    let mut client = RelayClient::new(
        info,
        vec![String::from("0.0.0.0"), String::from("w.konkito.com")],
    )
    .unwrap();

    println!("Create connection");

    client.step();
    let search = client.search(Search {
        session: 0,
        client: SearchType::None,
        name: SearchType::None,
        other: SearchType::None,
    });

    let search = search.get();

    println!("Search: {:?}", search);

    for adress in search {
        let mut where_is = client.where_is_adress(&adress);
        let Some(where_is) = where_is.pop() else{continue};
        let info = client.get(where_is).unwrap().info(&adress);
        let client_info = info.get();
        if let Some(info) = client_info {
            println!("Client: {:?}", info);
        }
    }

    let mut connections = Vec::new();
    let mut thread: Option<JoinHandle<(Adress, Conn)>> = None;

    let socket = Socket::new(Domain::IPV4, socket2::Type::DGRAM, None).unwrap();
    socket.bind(&SockAddr::from(
        "0.0.0.0:0".to_socket_addrs().unwrap().next().unwrap(),
    ));
    let mut connecting_to = Vec::new();

    let search = client.search(Search::default()).get();
    for adress in search {
        if adress != client.get(0).unwrap().adress() && !connecting_to.contains(&adress) {
            connecting_to.push(adress.clone());
            println!("Cannecting to: {:?}", adress);
            client.get(0).unwrap().request(&adress, String::new());
        }
    }

    loop {
        client.step();
        if let Some(worker) = thread.take() {
            if worker.is_finished() {
                let res = worker.join().unwrap();
                let mut buffer = [MaybeUninit::new(0); 1024];
                res.1.set_nonblocking(false).unwrap();
                res.1.send(b"Hello There").unwrap();
                let len = res.1.recv(&mut buffer).unwrap();
                let buffer: &[u8] = unsafe { std::mem::transmute(&buffer[0..len]) };
                let message = String::from_utf8(buffer.to_vec()).unwrap();
                println!("Message: {}", message);
                connections.push(res);
                return;
            } else {
                thread = Some(worker)
            }
        }

        if let Some((_, step)) = client.has_new() {
            match step {
                relay_man::client::response::RequestStage::NewRequest(new) => {
                    connecting_to.push(new.from.clone());
                    println!("New from: {:?}", new.from);
                    new.accept(true);
                }
                relay_man::client::response::RequestStage::NewRequestResponse(new) => {
                    println!("Res from: {:?}", new.from);
                    let res = new.add_socket(&socket);
                    match res {
                        relay_man::client::response::RegisterResponse::Success { port } => {
                            println!("Connection on port: {port}")
                        }
                        relay_man::client::response::RegisterResponse::Error => {
                            panic!("Some thing went rong!")
                        }
                    }
                    new.accept(true, Some(Duration::from_secs(2).as_nanos()));
                }
                relay_man::client::response::RequestStage::NewRequestFinal(new) => {
                    println!("Final from: {:?}", new.from);
                    let res = new.add_socket(&socket);
                    match res {
                        relay_man::client::response::RegisterResponse::Success { port } => {
                            println!("Connection on port: {port}")
                        }
                        relay_man::client::response::RegisterResponse::Error => {
                            panic!("Some thing went rong!")
                        }
                    }
                }
                relay_man::client::response::RequestStage::ConnectOn(new) => {
                    println!("ConnectOn: {:?}", new);
                    let socket = socket.try_clone().unwrap();
                    thread = Some(std::thread::spawn(|| {
                        (
                            new.adress.clone(),
                            new.connect(Duration::from_secs(5), Duration::from_millis(100), socket)
                                .unwrap(),
                        )
                    }));
                }
            }
        }
    }
}
