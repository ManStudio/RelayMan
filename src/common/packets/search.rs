use bytes_kman::prelude::*;

#[derive(Bytes, Clone, Debug)]
pub enum SearchType<T> {
    Fuzzy(T),
    Exact(T),
    None,
}

#[derive(Bytes, Clone, Debug)]
pub struct Search {
    pub session: u128,
    pub client: SearchType<String>,
    pub name: SearchType<String>,
    pub other: SearchType<Vec<u8>>,
}
