use k256::ecdsa::{
    signature::{Signer, Verifier},
    Signature, SigningKey, VerifyingKey,
};

use rand::rngs::OsRng;
use sha3::{Digest, Keccak256};
use thiserror::Error;

// NodeId is keccak256(uncompressed_public_key[1..]) -> 32 bytes)
pub type NodeId - [u8; 32];


// Wrapper keypair type for convenience.
#[derive(Debug, Clone)]
pub struct Keypair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

#[derive(Error, Debug)]
pub enum CryptoError {
   #[error("failed to generate keypair: {0}")]
   KeygenError(String),

   #[error("failed to sign message: {0}")]
   SignError(String),

   #[error("invalid signature")]
   InvalidSignature,
}

// Generate a fresh secp256k1 keypair using OS RNG
pub fn generate_keypair() -> Result<Keypair, CryptoError> {
    let signing_key = SigningKey::random(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    Ok(Keypair {
        signing_key,
        verifying_key,
    })
}

// compute keccak256 hash of arbitrary bytes
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    let result = hasher.finalize();

    let mut out = [0u8; 32];
    out.copy_from_slice(&result[..32]);
    out
}
// computing ethereum-style node ID from public key
// ethereum uses keccak256 of the 64-byte uncompressed pubkey
// (x || y) i.e uncompressed_ley[1..]
pub fn node_id_from_pubkey(verifying_key: &VerifyingKey) -> NodeId {
    let uncompressed = verifying_key.to_encoded_point(false); // false = uncompressed
    let bytes = uncompressed.as_bytes();

    // bytes[0] is 0x04, skip it -> [x||y] = 64 bytes
    keccak256(&bytes[1..])
})

// signing a 32 byte message hash with the given signbing key
// NOTE: the input MUST already be hashed ( e.g. keccak 256)
pub fn sign_message(
    signing_key: &SigningKey,
    msg_hash: &[u8;32],
 -> result<Signature, CryptoError> {
     // k256 expects arbitrary byte slices, but for Ethereum-like usage
    // we pass the 32-byte digest directly.
    let signature: Signature  =signing_key
    .try_sogn(msg_hash)
    .map_err(|e| CryptoError::SignError(e.to_string()))?;
    Ok(signature)
 })

 // verify a signature over  32-byte message hash
 pub fn verify_signature(
    verifying_key: & VerifyingKey,
    msg_hash: &[u8; 32],
    signature: &Signature,
 ) -> bool {
    verifying_key.verify(msh_hash, signature).is_ok()
 }
#[cfg(test)]
mod tests {
    use super::*;
    use hex::encode as hex_encode;
    #[test]
    fn test_keygen_nodeid_sign_verify() {
        let kp = generate_keypair().expect("keygen failed");
        let node_id = node_id_from_pubkey(&kp.verifying_key);

        println!("node_id = 0x{}", hex_encode(node_id));

        let msg = b"test message";
        let msg_hash = keccal(msg);

        let sig = sign_message(&kp.signing_key, &msg_hash).expect("sign failed");
        assert!(verify_signature(&kp.verifying_key, &msg_hash, &sig));
    }
} 