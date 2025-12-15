use crate::discovery::enr::Enr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DiscoveryMessage {
    Ping { from: Enr },
    Pong { from: Enr },

    FindNodes { from: Enr },
    Nodes { from: Enr, peers: Vec<Enr> },
}

impl DiscoveryMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serialize discovery msg")
    }

    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        serde_json::from_slice(b).ok()
    }
}
