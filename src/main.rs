mod transport;
mod discovery;
mod session;
mod protocol;
use discovery::{enr::Enr, service::DiscoveryService};
use transport::quic::endpoint::start_endpoint;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // usage:
    // cargo run -- <port> [bootstrap_port]
    let port: u16 = args[1].parse().unwrap();

    let endpoint = start_endpoint(port);
    let local_enr = Enr::new_local(port);

    let bootstrap = if args.len() > 2 {
        let bp: u16 = args[2].parse().unwrap();
        Some(format!("127.0.0.1:{bp}").parse().unwrap())
    } else {
        None
    };

    let svc = DiscoveryService::new(endpoint, local_enr);
    svc.run(bootstrap).await;
}
