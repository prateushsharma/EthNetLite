mod transport;

use std::net::SocketAddr;
use transport::quic::{
    connection::{accept, connect},
    endpoint::start_endpoint,
    stream::send,
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args[1].parse().unwrap();

    let endpoint = start_endpoint(port);

    if args.len() > 2 {
        let peer_port: u16 = args[2].parse().unwrap();
        let peer_addr = SocketAddr::from(([127, 0, 0, 1], peer_port));

        let conn = connect(&endpoint, peer_addr).await;
        send(&conn, b"hello from peer").await;
    }

    accept(endpoint).await;
}
