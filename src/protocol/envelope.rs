use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub proto: String,     // "discv-lite/0.1" | "mini-sync/0.1"
    pub data: Vec<u8>,     // JSON bytes for that protocol
}

impl Envelope {
    pub fn new(proto: impl Into<String>, data: Vec<u8>) -> Self {
        Self { proto: proto.into(), data}
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serialize envelope")
    }

    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        serde_json::from_slice(b).ok()
    }
}