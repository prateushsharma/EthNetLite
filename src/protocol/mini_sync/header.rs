use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub parent_hash: String,
    pub hash: String,
    pub number: u64,
}
