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

        // in connect every connection will be double but when connecting will be consumed and
        // seccond time nothing will happend!

        'm: for conn in connect {
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
                    // cache the first client
                    // this is ok because the hole relay is single threded
                    break;
                }
            }

            if let Some(connecting_to) = connecting_to {
                for (i, client) in self.clients.iter().enumerate() {
                    if client.session == conn.1 {
                        let mut finded = false;
                        for conn_to in client.to_connect.iter() {
                            if conn_to.session() == connecting_to {
                                finded = true;
                                break;
                            }
                        }
                        if !finded {
                            continue 'm;
                        }
                        index2 = i;
                        // cache the the seccond client
                        // this is ok because the hole relay is single threded
                        break;
                    }
                }
            } else {
                continue;
            }

            let port1;
            let port2;
            let adress1;
            let adress2;
            let addr1;
            let addr2;

            if let Some(client) = self.clients.get_mut(index1) {
                port1 = client.ports.pop();
                adress1 = client.from.clone();
                addr1 = client.adress.clone();
            } else {
                continue;
            }

            if let Some(client) = self.clients.get_mut(index2) {
                port2 = client.ports.pop();
                adress2 = client.from.clone();
                addr2 = client.adress.clone();
            } else {
                if let Some(port1) = port1 {
                    if let Some(client) = self.clients.get_mut(index1) {
                        client.ports.push(port1)
                    }
                }
                continue;
            }

            let Some(port1) = port1 else {
                if let Some(port2) = port2{
                    if let Some(client) = self.clients.get_mut(index2) {
                        client.ports.push(port2);
                    }
                }
                continue
            };
            let Some(port2) = port2 else {
                if let Some(client) = self.clients.get_mut(index1){
                   client.ports.push(port1);
                }
                continue
            };

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
        }
    }
}
