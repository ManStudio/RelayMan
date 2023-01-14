use std::time::SystemTime;

use bytes_kman::TBytes;

use crate::common::packets::{ConnectOn, Packets};

use super::{ClientStage, Connecting, RelayServer};

impl RelayServer {
    pub(crate) fn connect(&mut self) {
        let mut connect = Vec::new();

        for client in self.clients.iter() {
            if let ClientStage::Registered(rclient) = &client.stage {
                if !rclient.to_connect.is_empty() {
                    if !rclient.ports.is_empty() {
                        for to_conn in rclient.to_connect.iter() {
                            match to_conn {
                                Connecting::Finishing(session, time_offset) => {
                                    connect.push((client.session, *session, *time_offset));
                                }
                                _ => {}
                            }
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
                    if let ClientStage::Registered(rclient) = &client.stage {
                        for conn_to in rclient.to_connect.iter() {
                            if conn_to.session() == conn.1 {
                                connecting_to = Some(client.session);
                                break;
                            }
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

                        if let ClientStage::Registered(rclient) = &client.stage {
                            for conn_to in rclient.to_connect.iter() {
                                if conn_to.session() == connecting_to {
                                    finded = true;
                                    break;
                                }
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
            let private_adress1;
            let private_adress2;
            let addr1;
            let addr2;

            if let Some(client) = self.clients.get_mut(index1) {
                if let ClientStage::Registered(rclient) = &mut client.stage {
                    port1 = rclient.ports.pop();
                    adress1 = client.from.clone();
                    addr1 = rclient.adress.clone();
                    private_adress1 = rclient.private_adress.clone();
                } else {
                    continue;
                }
            } else {
                continue;
            }

            if let Some(client) = self.clients.get_mut(index2) {
                if let ClientStage::Registered(rclient) = &mut client.stage {
                    port2 = rclient.ports.pop();
                    adress2 = client.from.clone();
                    addr2 = rclient.adress.clone();
                    private_adress2 = rclient.private_adress.clone();
                } else {
                    continue;
                }
            } else {
                if let Some(port1) = port1 {
                    if let Some(client) = self.clients.get_mut(index1) {
                        if let ClientStage::Registered(rclient) = &mut client.stage {
                            rclient.ports.push(port1)
                        } else {
                            continue;
                        }
                    }
                }
                continue;
            }

            let Some(port1) = port1 else {
                if let Some(port2) = port2{
                    if let Some(client) = self.clients.get_mut(index2) {

                        if let ClientStage::Registered(rclient) = &mut client.stage {
                        rclient.ports.push(port2);
                        }else{continue}
                    }
                }
                continue
            };
            let Some(port2) = port2 else {
                if let Some(client) = self.clients.get_mut(index1){

                        if let ClientStage::Registered(rclient) = &mut client.stage {
                   rclient.ports.push(port1);
                    }else{
                        continue
                    }
                }
                continue
            };

            if let Some(client) = self.clients.get_mut(index1) {
                if let ClientStage::Registered(rclient) = &mut client.stage {
                    rclient
                        .to_connect
                        .retain(|to_conn| to_conn.session() != conn.1);
                }
            }
            if let Some(client) = self.clients.get_mut(index2) {
                if let ClientStage::Registered(rclient) = &mut client.stage {
                    rclient
                        .to_connect
                        .retain(|to_conn| to_conn.session() != conn.0);
                }
            }

            let adress1 = adress1.as_socket().unwrap().ip();
            let adress2 = adress2.as_socket().unwrap().ip();

            let has_the_same_ip = adress2 == adress1;

            let (adress1, adress2) = if has_the_same_ip {
                (private_adress1, private_adress2)
            } else {
                (adress1.to_string(), adress2.to_string())
            };

            let time = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                + conn.2;

            let pak = ConnectOn {
                session: conn.0,
                to: format!("{}:{}", adress2, port2),
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
                to: format!("{}:{}", adress1, port1),
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
