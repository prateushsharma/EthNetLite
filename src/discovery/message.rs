use crate::enr::EnrRecord;
use serde::{Deserialize, Serialize};
use k256::ecdsa::Signature;

// Serializable ENR for network messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireEnr {
    pub signature: Vec<u8>,
    pub seq: u64,
    pub pairs: Vec<(Vec<u8>, Vec<u8>)>,
}
impl From<&EnrRecord> for WireEnr {
    fn from(enr: &EnrRecord) -> Self {
        Self {
            signature: enr.signature.to_der().as_bytes().to_vec(),
            seq: enr.seq,
            pairs: enr.pairs.clone(),
        }
    }
}

impl TryFrom<WireEnr> for EnrRecord {
    type Error = String;

    fn try_from(value: WireEnr) -> Result<Self, Self::Error> {
        let signature = Signature::from_der(&value.signature)
            .map_err(|e| format!("invalid ENR signature: {}", e))?;
        Ok(Self {
            signature,
            seq: value.seq,
            pairs: value.pairs,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DiscoveryMessage {
    Ping { from: WireEnr },
    Pong { from: WireEnr },
    FindNodes { from: WireEnr },
    Nodes { from: WireEnr, peers: Vec<WireEnr> },
}

impl DiscoveryMessage {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serialize discovery msg")
    }

    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        serde_json::from_slice(b).ok()
    }
}
