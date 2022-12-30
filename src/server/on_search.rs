use bytes_kman::TBytes;

use crate::common::packets::{Packets, Search, SearchResponse, SearchType};

use super::{ClientStage, RelayServer};

impl RelayServer {
    pub(crate) fn on_search(&mut self, index: usize, search: Search) {
        let session;
        if let Some(client) = self.clients.get(index) {
            session = client.session;
        } else {
            return;
        }

        let mut adresses = Vec::new();

        for client in self.clients.iter() {
            if let ClientStage::Registered(rclient) = &client.stage {
                let mut valid = true;

                match &search.name {
                    SearchType::Fuzzy(name) => {
                        if !rclient.name.contains(name) {
                            break;
                        }
                        valid = false;
                    }
                    SearchType::Exact(name) => {
                        if rclient.name != *name {
                            valid = false;
                        }
                    }
                    SearchType::None => {}
                };

                match &search.client {
                    SearchType::Fuzzy(name) => {
                        if !rclient.client.contains(name) {
                            break;
                        }
                        valid = false;
                    }
                    SearchType::Exact(name) => {
                        if rclient.client != *name {
                            valid = false;
                        }
                    }
                    SearchType::None => {}
                };

                match &search.other {
                    SearchType::Fuzzy(name) => {
                        let mut finds = 0usize;

                        for by in &rclient.other {
                            if let Some(b) = name.get(finds) {
                                if *by == *b {
                                    finds += 1;
                                } else {
                                    finds = 0;
                                }
                                if finds == rclient.other.len() {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }

                        if name.len() < finds {
                            valid = false
                        }
                    }
                    SearchType::Exact(name) => {
                        if rclient.other != *name {
                            valid = false;
                        }
                    }
                    SearchType::None => {}
                };

                if valid {
                    adresses.push(rclient.adress.clone());
                }
            }
        }

        let pak = Packets::SearchResponse(SearchResponse { session, adresses });
        let mut bytes = pak.to_bytes();
        bytes.reverse();

        let _ = self.clients.get_mut(index).unwrap().conn.send(&bytes);
    }
}
