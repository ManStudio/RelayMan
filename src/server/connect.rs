use std::time::SystemTime;

use bytes_kman::TBytes;

use crate::common::packets::{ConnectOn, Packets};

use super::{Connecting, RelayServer};

impl RelayServer {
    pub(crate) fn connect(&mut self) {
        let mut connect = Vec::new();

        for client in self.clients.iter() {
            if !client.to_connect.is_empty() {
                if !client.ports.is_empty() {
                    for to_conn in client.to_connect.iter() {
                        match to_conn {
                            Connecting::Finishing(session) => {
                                connect.push((client.session, *session));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        for conn in connect {
            let mut index1 = 0;
            let mut index2 = 0;

            let mut connecting_to = None;
            for (i, client) in self.clients.iter().enumerate() {
                if client.session == conn.0 {
                    for conn_to in client.to_connect.iter() {
                        if conn_to.session() == conn.1 {
                            connecting_to = Some(client.session);
                            break;
                        }
                    }
                    index1 = i;
                    break;
                }
            }

            let mut is_falid = false;

            if let Some(connecting_to) = connecting_to {
                for (i, client) in self.clients.iter().enumerate() {
                    if client.session == conn.1 {
                        for conn_to in client.to_connect.iter() {
                            if conn_to.session() == connecting_to {
                                is_falid = true;
                                index2 = i;
                                break;
                            }
                        }
                    }
                }
            } else {
                continue;
            }

            if !is_falid {
                continue;
            }

            let mut port1 = None;
            let mut port2 = None;
            let mut adress1 = None;
            let mut adress2 = None;
            let mut addr1 = None;
            let mut addr2 = None;

            for client in self.clients.iter_mut() {
                if client.session == conn.0 {
                    port1 = client.ports.pop();
                    adress1 = Some(client.from.clone());
                    addr1 = Some(client.adress.clone());
                } else if client.session == conn.1 {
                    port2 = client.ports.pop();
                    adress2 = Some(client.from.clone());
                    addr2 = Some(client.adress.clone());
                } else {
                    continue;
                }

                if port1.is_some() && port2.is_some() {
                    break;
                }
            }

            if let Some(port1) = port1 {
                if let Some(port2) = port2 {
                    if let Some(client) = self.clients.get_mut(index1) {
                        client
                            .to_connect
                            .retain(|to_conn| to_conn.session() != conn.1);
                    }
                    if let Some(client) = self.clients.get_mut(index2) {
                        client
                            .to_connect
                            .retain(|to_conn| to_conn.session() != conn.0);
                    }

                    let Some(adress1) = adress1 else{continue};
                    let Some(adress2) = adress2 else{continue};
                    let Some(addr1) = addr1 else{continue};
                    let Some(addr2) = addr2 else{continue};

                    let time = SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                        + 10000000000;

                    let pak = ConnectOn {
                        session: conn.0,
                        to: format!("{}:{}", adress2.as_socket().unwrap().ip(), port2),
                        port: port1,
                        adress: addr2,
                        time,
                    };

                    let mut bytes = Packets::ConnectOn(pak).to_bytes();
                    bytes.reverse();
                    if let Some(client) = self.clients.get_mut(index1) {
                        let _ = client.conn.send(&bytes);
                    }

                    let pak = ConnectOn {
                        session: conn.1,
                        to: format!("{}:{}", adress1.as_socket().unwrap().ip(), port1),
                        port: port2,
                        adress: addr1,
                        time,
                    };

                    let mut bytes = Packets::ConnectOn(pak).to_bytes();
                    bytes.reverse();
                    if let Some(client) = self.clients.get_mut(index2) {
                        let _ = client.conn.send(&bytes);
                    }
                } else {
                    if let Some(client) = self.clients.get_mut(index1) {
                        if client.session == conn.0 {
                            client.ports.push(port1);
                            break;
                        }
                    }
                }
            } else {
                if let Some(port2) = port2 {
                    if let Some(client) = self.clients.get_mut(index2) {
                        if client.session == conn.1 {
                            client.ports.push(port2);
                            break;
                        }
                    }
                }
            }
        }
    }
}
