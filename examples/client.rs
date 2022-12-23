use std::time::Duration;

use rand::random;
use relay::{
    client::{ConnectionInfo, RelayClient},
    common::packets::{Search, SearchType},
};

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
    let mut client = RelayClient::new(info, vec![String::from("localhost")]).unwrap();

    println!("Create connection");

    client.step();
    client.search_request(Search {
        session: 0,
        client: SearchType::None,
        name: SearchType::None,
        other: SearchType::None,
    });

    client.step();
    std::thread::sleep(Duration::from_millis(200));
    client.step();

    let search = client.search();
    println!("Search: {:?}", search);

    for adress in search {
        let mut where_is = client.where_is_adress(&adress);
        let Some(where_is) = where_is.pop() else{continue};

        client.get_info_request(where_is, &adress);
        std::thread::sleep(Duration::from_millis(200));
        client.step();
        let client_info = client.get_info(where_is, &adress);
        if let Some(info) = client_info {
            println!("Client: {:?}", info);
        }
    }

    loop {
        client.step();
        std::thread::sleep(Duration::from_secs(1));
    }
}
