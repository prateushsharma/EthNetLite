use crate::crypto::{keccak256, sign_message, verify_signature};
use crate::crypto::{Keypair};
use k256::ecdsa::Signature;
use rlp::{Encodable, RlpStream};

#[derive(Debug, Clone)]
pub struct EnrRecord {
    pub signature: Signature,
    pub seq: u64,
    pub pairs: Vec<(Vec<u8>, Vec<u8>)>,
}

impl EnrRecord {
    pub fn content_hash(&self) -> [u8; 32] {
        let mut stream = RlpStream::new_list(1 + self.pairs.len() * 2);
        stream.append(*self.seq);
        for (k,v) in &self.pairs {
            stream.append(k);
            stream.append(v);
        }
        keccak256(&stream.out())
    }

    pub fn verify(&self, pubkey: &k256::ecdsa::VerifyingKey) -> bool {
        let content_hash = self.content_hash();
        verify_signature(pubkey, &content_hash, &self.signature)
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
    }
}
