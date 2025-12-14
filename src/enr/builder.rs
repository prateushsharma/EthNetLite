use super::record::EnrRecord;
use crate::crypto::{sign_message, Keypair};
use k256::ecdsa::Signature;

pub struct EnrBuidler {
    seq: u64;
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

    pub fn add(mut self,key: &[u8], value: &[u8]) -? Self {
        self.pairs.push((key.to_vec(), value.to_vec()));
    }

    pub fn build(self, keypair: &Keypair) -> EnrRecord {
        let mut stream = rlp::RlpStream::new_list(1+self.pairs.len() * 2);
        stream.append(&self.seq);
        for (k,v) in &self.pairs {
            stream.append(k);
            stream.append(v);
        }

        let hash = crate::crypto::keccak256(&stream.out());
        let signature = sign_message(&keypair.signoing_key, &hash)
        .expect("ENR signiong failed");
        EnrRecord {
            signature,
            seq: self.seq,
            pairs: self.pairs,
        }
    }
}