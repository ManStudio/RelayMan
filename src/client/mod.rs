use std::sync::{Arc, RwLock};

use crate::common::{
    adress::Adress,
    packets::{Packets, Search},
};

mod connection;
pub mod response;
pub use connection::*;

use self::response::{RequestStage, Response};

pub struct RelayClient {
    pub connections: Vec<Arc<RwLock<Connection>>>,
    pub connection_errors: Vec<ConnectionError>,
    pub info: ConnectionInfo,
}

#[derive(Debug)]
pub enum RelayClientError {
    ConnectionError(ConnectionError),
    NoRelays,
    NoConnections,
}

impl RelayClient {
    pub fn new(info: ConnectionInfo, relays: Vec<String>) -> Result<Self, RelayClientError> {
        let mut connection_errors = Vec::new();
        use RelayClientError::*;
        if relays.is_empty() {
            return Err(NoRelays);
        }

        let mut connections = Vec::new();
        for relay in relays {
            match Connection::new(relay, info.clone()) {
                Ok(conn) => {
                    connections.push(Arc::new(RwLock::new(conn)));
                }
                Err(error) => {
                    connection_errors.push(error);
                }
            }
        }

        if connections.is_empty() {
            println!("Errors: {:?}", connection_errors);
            return Err(NoConnections);
        }

        Ok(Self {
            connections,
            info,
            connection_errors,
        })
    }

    pub fn step(&mut self) {
        for conn in self.connections.iter_mut() {
            conn.step();
        }
    }

    pub fn where_is_adress(&self, adress: &Adress) -> Vec<usize> {
        let mut indexs = Vec::new();
        for (index, conn) in self.connections.iter().enumerate() {
            if conn.read().unwrap().adresses.contains(adress) {
                indexs.push(index);
            }
        }
        indexs
    }

    pub fn search(
        &self,
        search: Search,
    ) -> Response<Vec<Response<Box<dyn TConnection>, response::SearchResponse>>, Vec<Adress>> {
        let mut responses = Vec::new();
        for conn in self.connections.iter() {
            responses.push(conn.search(search.clone()))
        }

        Response {
            connection: responses,
            packets: Packets::Search(search),
            fn_has: search_fn_has,
            fn_get: search_fn_get,
        }
    }

    pub fn has_new(&self) -> Option<(usize, RequestStage)> {
        for (index, conn) in self.connections.iter().enumerate() {
            if let Some(new) = conn.has_new() {
                return Some((index, new));
            }
        }

        None
    }

    pub fn get(&self, index: usize) -> Option<&dyn TConnection> {
        if let Some(conn) = self.connections.get(index) {
            Some(conn)
        } else {
            None
        }
    }
}

// Search

fn search_fn_has(
    connections: &Vec<Response<Box<dyn TConnection>, response::SearchResponse>>,
    _: &Packets,
) -> bool {
    let mut count = 0;

    for conn in connections.iter() {
        count += conn.has() as usize
    }

    count == connections.len()
}

fn search_fn_get(
    connections: Vec<Response<Box<dyn TConnection>, response::SearchResponse>>,
    _: Packets,
) -> Vec<Adress> {
    let mut res = Vec::new();

    for conn in connections {
        let v = conn.get();
        for adress in v.adresses {
            if !res.contains(&adress) {
                res.push(adress)
            }
        }
    }

    res
}
