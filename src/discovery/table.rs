use crate::discovery::message::SerializableEnr;
use std::collections::HashMap;

#[derive(Debug)]
pub struct PeerTable {
    max_size: usize,
    peers: HashMap<String, SerializableEnr>, // node_id -> enr
}

impl PeerTable {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            peers: HashMap::new(),
        }
    }

    pub fn insert(&mut self, local: &SerializableEnr, enr: SerializableEnr) -> bool {
        if enr.node_id == local.node_id {
            return false;
        }
        if self.peers.contains_key(&enr.node_id) {
            return false;
        }
        if self.peers.len() >= self.max_size {
            // simplest eviction: drop an arbitrary peer
            if let Some(k) = self.peers.keys().next().cloned() {
                self.peers.remove(&k);
            }
        }
        self.peers.insert(enr.node_id.clone(), enr);
        true
    }

    pub fn insert_many(&mut self, local: &SerializableEnr, enrs: Vec<SerializableEnr>) -> Vec<SerializableEnr> {
        let mut added = vec![];
        for e in enrs {
            if self.insert(local, e.clone()) {
                added.push(e);
            }
        }
        added
    }

    pub fn list(&self) -> Vec<SerializableEnr> {
        self.peers.values().cloned().collect()
    }
}
