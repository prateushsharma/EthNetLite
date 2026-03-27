use crate::enr::EnrRecord;
use serde::{Deserialize, Serialize};

// Serializable ENR for network messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableEnr {
    pub node_id: String,
    pub ip: String,
    pub udp_port: u16,
    pub quic_port: u16,
    pub capabilities: Vec<String>,
}

impl From<&EnrRecord> for SerializableEnr {
    fn from(enr: &EnrRecord) -> Self {
        Self {
            node_id: enr.node_id().unwrap_or_default(),
            ip: enr.ip().unwrap_or_default(),
            udp_port: enr.udp_port().unwrap_or(0),
            quic_port: enr.quic_port().unwrap_or(0),
            capabilities: enr.capabilities(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DiscoveryMessage {
    Ping { from: SerializableEnr },
    Pong { from: SerializableEnr },
    FindNodes { from: SerializableEnr },
    Nodes { from: SerializableEnr, peers: Vec<SerializableEnr> },
}

impl DiscoveryMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serialize discovery msg")
    }

    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        serde_json::from_slice(b).ok()
    }
}
