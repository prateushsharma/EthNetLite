use crate::crypto::{keccak256, verify_signature};
use k256::ecdsa::{Signature, VerifyingKey};
use rlp::RlpStream;
#[derive(Debug, Clone)]
pub struct EnrRecord {
    pub signature: Signature,
    pub seq: u64,
    pub pairs: Vec<(Vec<u8>, Vec<u8>)>,
}

impl EnrRecord {
    pub fn content_hash(&self) -> [u8; 32] {
        let mut stream = RlpStream::new_list(1 + self.pairs.len() * 2);
        stream.append(&self.seq);
        for (k, v) in &self.pairs {
            stream.append(k);
            stream.append(v);
        }
        keccak256(&stream.out())
    }

    pub fn verify(&self, pubkey: &k256::ecdsa::VerifyingKey) -> bool {
        let content_hash = self.content_hash();
        verify_signature(pubkey, &content_hash, &self.signature)
    }

    pub fn pubkey_bytes(&self) -> Option<Vec<u8>> {
    self.get(b"secp256k1").cloned()
}

pub fn verifying_key(&self) -> Option<VerifyingKey> {
    let bytes = self.pubkey_bytes()?;
    VerifyingKey::from_sec1_bytes(&bytes).ok()
}

pub fn verify_self(&self) -> bool {
    let Some(pubkey) = self.verifying_key() else {
        return false;
    };
    self.verify(&pubkey)
}

    // Convenience getters for common ENR fields
    pub fn node_id(&self) -> Option<String> {
        // Node ID is derived from the public key, but for now we'll look for "id" field
        self.get(b"id").map(|v| String::from_utf8_lossy(v).to_string())
    }

    pub fn ip(&self) -> Option<String> {
        self.get(b"ip").map(|v| String::from_utf8_lossy(v).to_string())
    }

    pub fn udp_port(&self) -> Option<u16> {
        self.get(b"udp").and_then(|v| {
            if v.len() == 2 {
                Some(u16::from_be_bytes([v[0], v[1]]))
            } else {
                None
            }
        })
    }

    pub fn quic_port(&self) -> Option<u16> {
        self.get(b"quic").and_then(|v| {
            if v.len() == 2 {
                Some(u16::from_be_bytes([v[0], v[1]]))
            } else {
                None
            }
        })
    }

    pub fn capabilities(&self) -> Vec<String> {
        self.get_all(b"cap").iter().filter_map(|v| {
            String::from_utf8((*v).clone()).ok()
        }).collect()
    }

    pub fn seq(&self) -> u64 {
        self.seq
    }

    fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    fn get_all(&self, key: &[u8]) -> Vec<&Vec<u8>> {
        self.pairs.iter().filter(|(k, _)| k == key).map(|(_, v)| v).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_keypair;

    #[test]
    fn test_enr_sign_and_verify() {
        let kp = generate_keypair().unwrap();

        let enr = super::super::builder::EnrBuilder::new()
            .add(b"id", b"v4")
            .add(b"ip", b"127.0.0.1")
            .add(b"quic", &9001u16.to_be_bytes())
            .build(&kp);

        assert!(enr.verify(&kp.verifying_key));
        assert!(enr.verify_self());
    }
}
