use quinn::{Connection, Endpoint};
use std::net::SocketAddr;

pub async fn connect(endpoint: &Endpoint, addr: SocketAddr) -> Connection {
    let conn = endpoint
        .connect(addr, "localhost")
        .unwrap()
        .await
        .unwrap();

    println!("[QUIC] connected to {}", addr);
    conn
}

pub async fn accept(endpoint: Endpoint) {
    while let Some(connecting) = endpoint.accept().await {
        let conn = connecting.await.unwrap();
        println!(
            "[QUIC] incoming connection from {}",
            conn.remote_address()
        );

        tokio::spawn(super::stream::handle_streams(conn));
    }
}
