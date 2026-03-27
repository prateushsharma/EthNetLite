use super::record::EnrRecord;
use crate::crypto::{sign_message, Keypair};
use k256::ecdsa::Signature;

pub struct EnrBuilder {
    seq: u64,
    pairs: Vec<(Vec<u8>, Vec<u8>)>,
}
impl EnrBuilder {
    pub fn new() -> Self {
        Self {
            seq: 1,
            pairs: Vec::new(),
        }
    }

    pub fn seq(mut self, seq: u64) -> Self {
        self.seq = seq;
        self
    }

    pub fn add(mut self, key: &[u8], value: &[u8]) -> Self {
        self.pairs.push((key.to_vec(), value.to_vec()));
        self
    }

    pub fn ip(mut self, ip: &str) -> Self {
        self.pairs.push((b"ip".to_vec(), ip.as_bytes().to_vec()));
        self
    }

    pub fn udp_port(mut self, port: u16) -> Self {
        self.pairs.push((b"udp".to_vec(), port.to_be_bytes().to_vec()));
        self
    }

    pub fn quic_port(mut self, port: u16) -> Self {
        self.pairs.push((b"quic".to_vec(), port.to_be_bytes().to_vec()));
        self
    }

    pub fn id(mut self, id: &str) -> Self {
        self.pairs.push((b"id".to_vec(), id.as_bytes().to_vec()));
        self
    }

    pub fn capability(mut self, cap: &str) -> Self {
        self.pairs.push((b"cap".to_vec(), cap.as_bytes().to_vec()));
        self
    }

    pub fn build(self, keypair: &Keypair) -> EnrRecord {
        let mut stream = rlp::RlpStream::new_list(1 + self.pairs.len() * 2);
        stream.append(&self.seq);
        for (k, v) in &self.pairs {
            stream.append(k);
            stream.append(v);
        }

        let content_hash = crate::crypto::keccak256(&stream.out());
        let signature = sign_message(&keypair.signing_key, &content_hash)
            .expect("ENR signing failed");
        EnrRecord {
            signature,
            seq: self.seq,
            pairs: self.pairs,
        }
    }
}