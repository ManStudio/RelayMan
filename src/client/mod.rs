use crate::common::{
    adress::Adress,
    packets::{NewRequest, Packets, RequestFinal, RequestResponse, Search},
};

mod connection;
pub use connection::*;

pub struct RelayClient {
    pub connections: Vec<Connection>,
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
        use RelayClientError::*;
        if relays.len() == 0 {
            return Err(NoRelays);
        }

        let mut connections = Vec::new();
        for relay in relays {
            match Connection::new(relay, info.clone()) {
                Ok(conn) => {
                    connections.push(conn);
                }
                Err(error) => {
                    return Err(ConnectionError(error));
                }
            }
        }

        if connections.len() == 0 {
            return Err(NoConnections);
        }

        Ok(Self { connections, info })
    }

    pub fn step(&mut self) {
        for conn in self.connections.iter_mut() {
            conn.step();
        }
    }

    pub fn search_request(&mut self, search: Search) {
        for conn in self.connections.iter_mut() {
            conn.adresses.clear();
            conn.send(Packets::Search(search.clone()));
        }
    }

    pub fn search(&mut self) -> Vec<Adress> {
        let mut adresses = Vec::new();

        for conn in self.connections.iter_mut() {
            for adress in conn.adresses.iter() {
                if !adresses.contains(adress) {
                    adresses.push(adress.clone())
                }
            }
        }

        adresses
    }

    pub fn where_is_adress(&self, adress: &Adress) -> Vec<usize> {
        let mut indexs = Vec::new();
        for (index, conn) in self.connections.iter().enumerate() {
            if conn.adresses.contains(adress) {
                indexs.push(index);
            }
        }
        indexs
    }

    pub fn get_info_request(&mut self, conn_index: usize, adress: &Adress) {
        if let Some(conn) = self.connections.get_mut(conn_index) {
            return conn.get_info_request(adress);
        }
    }

    pub fn get_info(&mut self, conn_index: usize, adress: &Adress) -> Option<ConnectionInfo> {
        if let Some(conn) = self.connections.get_mut(conn_index) {
            return conn.get_info(adress);
        }
        None
    }

    pub fn request(
        &mut self,
        conn_index: usize,
        adress: &Adress,
        secret: impl Into<String>,
    ) -> Option<()> {
        if let Some(conn) = self.connections.get_mut(conn_index) {
            conn.request(adress, secret);
            return Some(());
        }

        None
    }

    pub fn request_response(
        &mut self,
        conn_index: usize,
        adress: &Adress,
        secret: Option<impl Into<String>>,
    ) -> Option<()> {
        if let Some(conn) = self.connections.get_mut(conn_index) {
            conn.request_response(adress, secret);
            return Some(());
        }

        None
    }

    pub fn request_final(
        &mut self,
        conn_index: usize,
        adress: &Adress,
        accept: bool,
    ) -> Option<()> {
        if let Some(conn) = self.connections.get_mut(conn_index) {
            conn.request_final(adress, accept);
            return Some(());
        }

        None
    }

    pub fn has_new_request(&mut self) -> Option<(usize, NewRequest)> {
        for (index, conn) in self.connections.iter_mut().enumerate() {
            if let Some(new_request) = conn.has_new_request() {
                return Some((index, new_request));
            }
        }
        None
    }

    pub fn has_request_response(&mut self) -> Option<(usize, RequestResponse)> {
        for (index, conn) in self.connections.iter_mut().enumerate() {
            if let Some(request_response) = conn.has_request_response() {
                return Some((index, request_response));
            }
        }
        None
    }

    pub fn has_request_final(&mut self) -> Option<(usize, RequestFinal)> {
        for (index, conn) in self.connections.iter_mut().enumerate() {
            if let Some(request_final) = conn.has_request_final() {
                return Some((index, request_final));
            }
        }
        None
    }
}
