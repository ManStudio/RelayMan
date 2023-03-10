use bytes_kman::TBytes;

use crate::common::packets::{Info, InfoRequest, Packets};

use super::{ClientStage, RelayServer};

impl RelayServer {
    pub(crate) fn on_info(&mut self, index: usize, info: InfoRequest) {
        if let Some(client) = self.clients.get(index) {
            if client.session != info.session {
                return;
            }
        } else {
            return;
        }

        let mut pak = Info {
            has: false,
            name: String::new(),
            client: String::new(),
            other: Vec::new(),
            adress: vec![],
        };

        for client in self.clients.iter() {
            if let ClientStage::Registered(rclient) = &client.stage {
                if rclient.adress == info.adress {
                    pak.has = true;
                    pak.name = rclient.name.clone();
                    pak.client = rclient.client.clone();
                    pak.other = rclient.other.clone();
                    pak.adress = rclient.adress.clone();
                    break;
                }
            }
        }

        if let Some(client) = self.clients.get_mut(index) {
            let pak = Packets::Info(pak);
            let mut bytes = pak.to_bytes();
            bytes.reverse();

            let _ = client.conn.send(&bytes);
        }
    }
}
