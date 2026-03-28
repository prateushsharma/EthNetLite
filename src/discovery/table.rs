use crate::discovery::message::WireEnr;
use std::collections::HashMap;

#[derive(Debug)]
pub struct PeerTable {
    max_size: usize,
    peers: HashMap<String, WireEnr>, // node_id -> enr
}

impl PeerTable {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            peers: HashMap::new(),
        }
    }

    pub fn insert(&mut self, local: &WireEnr, enr: WireEnr) -> bool {
        let local_node_id = node_id(local);
        let remote_node_id = node_id(&enr);

        let (Some(local_node_id), Some(remote_node_id)) = (local_node_id, remote_node_id) else {
            return false;
        };

        if remote_node_id == local_node_id {
            return false;
        }
        if self.peers.contains_key(&remote_node_id) {
            return false;
        }
        if self.peers.len() >= self.max_size {
            if let Some(k) = self.peers.keys().next().cloned() {
                self.peers.remove(&k);
            }
        }

        self.peers.insert(remote_node_id, enr);
        true
    }

    pub fn insert_many(&mut self, local: &WireEnr, enrs: Vec<WireEnr>) -> Vec<WireEnr> {
        let mut added = vec![];
        for e in enrs {
            if self.insert(local, e.clone()) {
                added.push(e);
            }
        }
        added
    }

    pub fn list(&self) -> Vec<WireEnr> {
        self.peers.values().cloned().collect()
    }
}

fn node_id(enr: &WireEnr) -> Option<String> {
    enr.pairs
        .iter()
        .find(|(k, _)| k.as_slice() == b"id")
        .and_then(|(_, v)| String::from_utf8(v.clone()).ok())
}