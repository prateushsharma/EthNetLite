use serde::{Deserialize, Serialize};
use crate::protocol::mini_sync::header::Header;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub genesis_hash: String,
    pub head_hash: String,
    pub head_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestHeaders {
    pub start: u64,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Headers {
    pub headers: Vec<Header>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MiniSyncMessage {
    Status(Status),
    RequestHeaders(RequestHeaders),
    Headers(Headers),
}

impl MiniSyncMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serialize mini-sync msg")
    }
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        serde_json::from_slice(b).ok()
    }
}
