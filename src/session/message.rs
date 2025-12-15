use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub node_id: String,          // hex string (same style as ENR)
    pub protocol_version: u32,    // for MiniEthNet session layer
    pub capabilities: Vec<String>,// e.g. ["discv-lite/0.1", "mini-sync/0.1"]
    pub genesis: String,          // hex hash (placeholder for now)
    pub head_height: u64,         // placeholder for now
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloAck {
    pub node_id: String,
    pub agreed_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum SessionMessage {
    Hello(Hello),
    HelloAck(HelloAck),
}

impl SessionMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serialize session msg")
    }

    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        serde_json::from_slice(b).ok()
    }
}
