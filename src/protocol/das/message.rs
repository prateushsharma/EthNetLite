use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DasMessage {
    AnnounceData {
        data_id: String,
        total_chunks: u64,
    },
    RequestChunk {
        data_id: String,
        index: u64,
    },
    ChunkResponse {
        data_id: String,
        index: u64,
        bytes: Vec<u8>,
    },
}

impl DasMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serialize das msg")
    }

    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        serde_json::from_slice(b).ok()
    }
}