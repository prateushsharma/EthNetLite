// src/main.rs

mod transport;
mod discovery;
mod session;
mod protocol;
mod telemetry;

use std::sync::{atomic::AtomicU64, Arc, Mutex};

use discovery::{enr::Enr, service::DiscoveryService};
use transport::quic::endpoint::start_endpoint;
use protocol::mini_sync::manager::ChainManager;
use session::table::SessionTable;
use telemetry::event_bus::EventBus;
use telemetry::grpc_server::start_grpc_server;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // usage:
    //   cargo run -- <port>
    //   cargo run -- <port> <bootstrap_port>
    let port: u16 = args[1].parse().unwrap();

    // gRPC port convention: p2p + 1000
    // 9001 → 10001, 9002 → 10002, 9003 → 10003
    let grpc_port = port + 1000;

    let endpoint = start_endpoint(port);
    let local_enr = Enr::new_local(port);

    let bootstrap = if args.len() > 2 {
        let bp: u16 = args[2].parse().unwrap();
        Some(format!("127.0.0.1:{bp}").parse().unwrap())
    } else {
        None
    };

    // ── Shared state ──────────────────────────────────────────────────────────
    // Both gRPC server and P2P service point to the same heap allocation.
    // No copies. No sync overhead. One source of truth.
    let chain             = Arc::new(Mutex::new(ChainManager::new("0xgenesis".to_string())));
    let sessions          = Arc::new(Mutex::new(SessionTable::new()));
    let event_bus         = EventBus::new();
    let canonical_switches = Arc::new(AtomicU64::new(0));

    // ── Spawn gRPC server as background task ──────────────────────────────────
    {
        let chain   = chain.clone();
        let sessions = sessions.clone();
        let event_bus = event_bus.clone();
        let switches  = canonical_switches.clone();
        tokio::spawn(async move {
            start_grpc_server(grpc_port, chain, sessions, event_bus, switches).await;
        });
    }

    println!("[MAIN] p2p={} grpc={}", port, grpc_port);

    // ── Boot P2P service ──────────────────────────────────────────────────────
    let svc = DiscoveryService::new_with_state(
        endpoint,
        local_enr,
        chain,
        sessions,
        event_bus,
        canonical_switches,
    );

    svc.run(bootstrap).await;
}