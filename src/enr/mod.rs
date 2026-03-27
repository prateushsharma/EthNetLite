pub mod record;
pub mod builder;
pub mod error;

pub use record::EnrRecord;
pub use builder::EnrBuilder;

use crate::crypto::generate_keypair;

/// Create a local ENR for testing with default capabilities
pub fn create_local_enr(port: u16) -> (EnrRecord, crate::crypto::Keypair) {
    let keypair = generate_keypair().expect("Failed to generate keypair");

    // For now, use a simple ID. In production, this should be derived from pubkey
    let node_id = hex::encode(&keypair.verifying_key.to_encoded_point(false).as_bytes()[1..17]);

    let enr = EnrBuilder::new()
        .id(&format!("v4-{}", &node_id))
        .ip("127.0.0.1")
        .udp_port(port)
        .quic_port(port)
        .capability("discv-lite/0.1")
        .capability("mini-sync/0.1")
        .build(&keypair);

    (enr, keypair)
}