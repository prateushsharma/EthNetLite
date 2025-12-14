use crate::transport::quic::config::{client_config, server_config};
use quinn::Endpoint;
use std::net::SocketAddr;

pub fn start_endpoint(port: u16) -> Endpoint {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let mut endpoint = Endpoint::server(server_config(), addr).unwrap();
    endpoint.set_default_client_config(client_config());

    println!("[QUIC] listening on {}", addr);
    endpoint
}
