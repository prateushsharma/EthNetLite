use quinn::{ClientConfig, ServerConfig};
use rcgen::generate_simple_self_signed;
use rustls::{
    Certificate, PrivateKey,
    client::{ServerCertVerified, ServerCertVerifier},
};
use std::sync::Arc;
use std::time::SystemTime;

/// DEV ONLY — skip cert verification
struct SkipServerVerification;

impl ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &Certificate,
        _intermediates: &[Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp: &[u8],
        _now: SystemTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
}

pub fn server_config() -> ServerConfig {
    // rcgen 0.11 API (STABLE)
    let cert = generate_simple_self_signed(vec!["localhost".into()]).unwrap();

    let cert_chain = vec![Certificate(cert.serialize_der().unwrap())];
    let key = PrivateKey(cert.serialize_private_key_der());

    ServerConfig::with_single_cert(cert_chain, key).unwrap()
}

pub fn client_config() -> ClientConfig {
    let tls = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    ClientConfig::new(Arc::new(tls))
}
