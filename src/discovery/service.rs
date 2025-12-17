use crate::discovery::enr::Enr;
use crate::discovery::message::DiscoveryMessage;
use crate::discovery::table::PeerTable;

use crate::protocol::envelope::Envelope;
use crate::protocol::mini_sync::chain::Chain;
use crate::protocol::mini_sync::message::{MiniSyncMessage, RequestHeaders, Status};

use crate::session::handshake::{inbound_handshake, outbound_handshake};

use quinn::{Connection, Endpoint};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{sleep, Duration};

const DISC_PROTO: &str = "discv-lite/0.1";
const SYNC_PROTO: &str = "mini-sync/0.1";

pub struct DiscoveryService {
    endpoint: Endpoint,
    local_enr: Enr,
    table: Arc<Mutex<PeerTable>>,
    chain: Arc<Mutex<Chain>>,
    local_caps: Vec<String>,
}

impl DiscoveryService {
    pub fn new(endpoint: Endpoint, local_enr: Enr) -> Self {
        let genesis = "0xgenesis".to_string();
        Self {
            endpoint,
            local_enr,
            table: Arc::new(Mutex::new(PeerTable::new(32))),
            chain: Arc::new(Mutex::new(Chain::new(genesis))),
            local_caps: vec![DISC_PROTO.to_string(), SYNC_PROTO.to_string()],
        }
    }

    pub async fn run(self, bootstrap: Option<SocketAddr>) {
        println!(
            "[DISC] local ENR: node_id={} addr={}:{}",
            self.local_enr.node_id, self.local_enr.ip, self.local_enr.port
        );

        // -------- Inbound accept loop (ONE accept loop per connection) --------
        let accept_endpoint = self.endpoint.clone();
        let table_for_accept = self.table.clone();
        let chain_for_accept = self.chain.clone();
        let local_for_accept = self.local_enr.clone();
        let caps_for_accept = self.local_caps.clone();

        tokio::spawn(async move {
            loop {
                if let Some(connecting) = accept_endpoint.accept().await {
                    match connecting.await {
                        Ok(conn) => {
                            let table = table_for_accept.clone();
                            let chain = chain_for_accept.clone();
                            let local = local_for_accept.clone();
                            let caps = caps_for_accept.clone();

                            tokio::spawn(async move {
                                if let Ok(sess) = inbound_handshake(&conn, &local.node_id, &caps).await {
                                    println!(
                                        "[SESS] inbound with {} agreed={:?}",
                                        sess.remote_node_id, sess.agreed_caps
                                    );

                                    // start per-connection mux loop
                                    connection_loop(conn, local, table, chain).await;
                                }
                            });
                        }
                        Err(_) => {
                            // ignore accept errors (common for short-lived conns)
                        }
                    }
                }
            }
        });

        // -------- Bootstrap dial once --------
        if let Some(addr) = bootstrap {
            if let Ok(conn) = self.dial(addr).await {
                if let Ok(sess) = outbound_handshake(&conn, &self.local_enr.node_id, &self.local_caps).await {
                    println!("[SESS] outbound bootstrap {:?}", sess);

                    // Send discovery bootstrap msgs
                    let _ = send_enveloped(
                        &conn,
                        DISC_PROTO,
                        &DiscoveryMessage::Ping { from: self.local_enr.clone() }.to_bytes(),
                    )
                    .await;

                    let _ = send_enveloped(
                        &conn,
                        DISC_PROTO,
                        &DiscoveryMessage::FindNodes { from: self.local_enr.clone() }.to_bytes(),
                    )
                    .await;

                    // Send mini-sync Status
                    let st = self.local_status();
                    let _ = send_enveloped(&conn, SYNC_PROTO, &MiniSyncMessage::Status(st).to_bytes()).await;
                }
            }
        }

        // -------- Periodic refresh loop --------
        loop {
            self.refresh_round().await;
            sleep(Duration::from_secs(3)).await;
        }
    }

    fn local_status(&self) -> Status {
        let chain = self.chain.lock().unwrap();
        Status {
            genesis_hash: chain.headers[0].hash.clone(),
            head_hash: chain.head_hash(),
            head_number: chain.height(),
        }
    }

    async fn dial(&self, addr: SocketAddr) -> Result<Connection, quinn::ConnectionError> {
        let connecting = self.endpoint.connect(addr, "localhost").unwrap();
        let conn = connecting.await?;
        println!("[DISC] dialed {}", addr);
        Ok(conn)
    }

    async fn refresh_round(&self) {
        let peers = { self.table.lock().unwrap().list() };

        for p in peers {
            let addr: SocketAddr = format!("{}:{}", p.ip, p.port).parse().unwrap();
            if let Ok(conn) = self.dial(addr).await {
                if let Ok(sess) = outbound_handshake(&conn, &self.local_enr.node_id, &self.local_caps).await {
                    println!("[SESS] outbound {:?}", sess);

                    // discovery upkeep
                    let _ = send_enveloped(
                        &conn,
                        DISC_PROTO,
                        &DiscoveryMessage::Ping { from: self.local_enr.clone() }.to_bytes(),
                    )
                    .await;

                    let _ = send_enveloped(
                        &conn,
                        DISC_PROTO,
                        &DiscoveryMessage::FindNodes { from: self.local_enr.clone() }.to_bytes(),
                    )
                    .await;

                    // sync upkeep (send status)
                    let st = self.local_status();
                    let _ = send_enveloped(&conn, SYNC_PROTO, &MiniSyncMessage::Status(st).to_bytes()).await;
                }
            }
        }
    }
}

// ------------------ ONE connection loop that demuxes streams ------------------

async fn connection_loop(
    conn: Connection,
    local: Enr,
    table: Arc<Mutex<PeerTable>>,
    chain: Arc<Mutex<Chain>>,
) {
    loop {
        let Ok((_send, mut recv)) = conn.accept_bi().await else { return; };

        let Ok(len) = recv.read_u32().await else { continue; };
        let mut data = vec![0u8; len as usize];
        if recv.read_exact(&mut data).await.is_err() {
            continue;
        }

        let Some(env) = Envelope::from_bytes(&data) else { continue; };

        match env.proto.as_str() {
            DISC_PROTO => {
                handle_discovery_msg(&conn, &local, &table, &env.data).await;
            }
            SYNC_PROTO => {
                handle_sync_msg(&conn, &chain, &env.data).await;
            }
            _ => {
                // unknown proto → ignore
            }
        }
    }
}

// ------------------ Discovery handler (same logic as Module 4, just enveloped) ------------------

async fn handle_discovery_msg(
    conn: &Connection,
    local: &Enr,
    table: &Arc<Mutex<PeerTable>>,
    payload: &[u8],
) {
    let Some(msg) = DiscoveryMessage::from_bytes(payload) else { return; };

    match msg {
        DiscoveryMessage::Ping { from } => {
            println!("[DISC] PING from {}:{}", from.ip, from.port);
            table.lock().unwrap().insert(local, from.clone());

            let _ = send_enveloped(
                conn,
                DISC_PROTO,
                &DiscoveryMessage::Pong { from: local.clone() }.to_bytes(),
            )
            .await;
        }
        DiscoveryMessage::Pong { from } => {
            println!("[DISC] PONG from {}:{}", from.ip, from.port);
            table.lock().unwrap().insert(local, from);
        }
        DiscoveryMessage::FindNodes { from } => {
            println!("[DISC] FIND_NODES from {}:{}", from.ip, from.port);
            table.lock().unwrap().insert(local, from.clone());

            let peers = table.lock().unwrap().list();
            let _ = send_enveloped(
                conn,
                DISC_PROTO,
                &DiscoveryMessage::Nodes { from: local.clone(), peers }.to_bytes(),
            )
            .await;
        }
        DiscoveryMessage::Nodes { from, peers } => {
            println!("[DISC] NODES from {}:{} ({} peers)", from.ip, from.port, peers.len());
            table.lock().unwrap().insert(local, from);
            let added = table.lock().unwrap().insert_many(local, peers);
            if !added.is_empty() {
                println!("[DISC] added {} new peers", added.len());
            }
        }
    }
}

// ------------------ mini-sync handler (ETH/66-lite) ------------------

async fn handle_sync_msg(conn: &Connection, chain: &Arc<Mutex<Chain>>, payload: &[u8]) {
    let Some(msg) = MiniSyncMessage::from_bytes(payload) else { return; };

    match msg {
        MiniSyncMessage::Status(remote) => {
            let (local_height, local_head) = {
                let c = chain.lock().unwrap();
                (c.height(), c.head_hash())
            };

            // MVP fork-choice: if remote is longer, request missing range
            if remote.head_number > local_height && remote.genesis_hash == chain.lock().unwrap().headers[0].hash {
                let req = MiniSyncMessage::RequestHeaders(RequestHeaders {
                    start: local_height + 1,
                    count: remote.head_number - local_height,
                });
                let _ = send_enveloped(conn, SYNC_PROTO, &req.to_bytes()).await;
                println!(
                    "[SYNC] remote higher ({} > {}), requesting headers…",
                    remote.head_number, local_height
                );
            } else {
                // useful debug
                let _ = local_head;
            }
        }

        MiniSyncMessage::RequestHeaders(req) => {
            let headers = {
                let c = chain.lock().unwrap();
                c.headers
                    .iter()
                    .filter(|h| h.number >= req.start)
                    .take(req.count as usize)
                    .cloned()
                    .collect::<Vec<_>>()
            };

            let resp = MiniSyncMessage::Headers(crate::protocol::mini_sync::message::Headers { headers });
            let _ = send_enveloped(conn, SYNC_PROTO, &resp.to_bytes()).await;
        }

        MiniSyncMessage::Headers(hs) => {
            let mut c = chain.lock().unwrap();
            let before = c.height();
            c.append_linear(hs.headers);
            let after = c.height();

            if after > before {
                println!("[SYNC] advanced head {} -> {}", before, after);
            }
        }
    }
}

// ------------------ Shared framed sender (Envelope + length prefix) ------------------

async fn send_enveloped(conn: &Connection, proto: &str, payload: &[u8]) -> Result<(), ()> {
    let env = Envelope::new(proto.to_string(), payload.to_vec()).to_bytes();
    let (mut send, _) = conn.open_bi().await.map_err(|_| ())?;
    send.write_u32(env.len() as u32).await.map_err(|_| ())?;
    send.write_all(&env).await.map_err(|_| ())?;
    send.finish().await.map_err(|_| ())?;
    Ok(())
}
