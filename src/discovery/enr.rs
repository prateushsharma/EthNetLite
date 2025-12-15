use rand::RngCore;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize,Deserialize, PartialEq, Eq, Hash)]
pub struct Enr {
    pub node_id: String,
    pub ip: String,
    pub port: u16,
}

impl Enr {
    pub fn new_local(port: u16) -> Self {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);

        Self {
            node_id: hex::encode(bytes),
            ip: "127.0.0.1".to_string(),
            port,
        }
    }
}