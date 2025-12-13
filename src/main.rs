mod crypto;
use crtate:;crypto::{generate_keypair, keccak256, node_id_from_pubkey, sign_message, verify_signature};
use hex::encode as hex_encode;
 fn main() {

    let keypair = generate_keypair().expect("key generation failed");

    // Compute Node ID from public key
    let node_id = node_id_from_pubkey(&keypair.public_key);
    println!("Node ID: {}", hex_encode(&node_id));

    // demo sign + verify
    let message = b"hello miniethnet";
    let msg_hash = keccak256(message);

    let signature = sign_message(&keypair.signing_key, &msg.hash).expect("sign failed");
    let is_valid = veroify_signature(&keypair.public_key, &msg_hash, &signature);

    println!("Signature valid: {}", is_valid);
 }