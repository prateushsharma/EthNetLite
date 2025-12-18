use crate::discovery::enr::Enr;
use crate::discovery::message::DiscoveryMessage;
use crate::discovery::table::PeerTable;

use crate::protocol::envelope::Envelope;
use crate::protocol::mini_sync::manager::ChainManager;
use crate::protocol::mini_sync::message::{MiniSyncMessage, Status};
use crate::protocol::mini_sync::producer::start_header_producer;

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
    chain: Arc<Mutex<ChainManager>>,
    local_caps: Vec<String>,
}

impl DiscoveryService {
    pub fn new(endpoint: Endpoint, local_enr: Enr) -> Self {
        let genesis = "0xgenesis".to_string();
        Self {
            endpoint,
            local_enr,
            table: Arc::new(Mutex::new(PeerTable::new(32))),
            chain: Arc::new(Mutex::new(ChainManager::new(genesis))),
            local_caps: vec![DISC_PROTO.to_string(), SYNC_PROTO.to_string()],
        }
    }

    pub async fn run(self, bootstrap: Option<SocketAddr>) {
        // enable header producer on leader
        if self.local_enr.port == 9001 {
            start_header_producer(self.chain.clone(), 2).await;
            println!("[MINE] header producer enabled");
        }

        println!(
            "[DISC] local ENR: node_id={} addr={}:{}",
            self.local_enr.node_id, self.local_enr.ip, self.local_enr.port
        );

        // ---------------- inbound accept loop ----------------
        let ep = self.endpoint.clone();
        let table = self.table.clone();
        let chain = self.chain.clone();
        let local = self.local_enr.clone();
        let caps = self.local_caps.clone();

        tokio::spawn(async move {
            loop {
                if let Some(connecting) = ep.accept().await {
                    if let Ok(conn) = connecting.await {
                        let table = table.clone();
                        let chain = chain.clone();
                        let local = local.clone();
                        let caps = caps.clone();

                        tokio::spawn(async move {
                            if let Ok(sess) =
                                inbound_handshake(&conn, &local.node_id, &caps).await
                            {
                                println!(
                                    "[SESS] inbound {} agreed={:?}",
                                    sess.remote_node_id, sess.agreed_caps
                                );
                                connection_loop(conn, local, table, chain).await;
                            }
                        });
                    }
                }
            }
        });

        // ---------------- bootstrap ----------------
        if let Some(addr) = bootstrap {
            if let Ok(conn) = self.dial(addr).await {
                if let Ok(sess) =
                    outbound_handshake(&conn, &self.local_enr.node_id, &self.local_caps).await
                {
                    println!("[SESS] outbound bootstrap {:?}", sess);

                    let _ = send_enveloped(
                        &conn,
                        DISC_PROTO,
                        &DiscoveryMessage::Ping {
                            from: self.local_enr.clone(),
                        }
                        .to_bytes(),
                    )
                    .await;

                    let st = self.local_status();
                    let _ = send_enveloped(
                        &conn,
                        SYNC_PROTO,
                        &MiniSyncMessage::Status(st).to_bytes(),
                    )
                    .await;
                }
            }
        }

        // ---------------- refresh loop ----------------
        loop {
            self.refresh_round().await;
            sleep(Duration::from_secs(3)).await;
        }
    }

    fn local_status(&self) -> Status {
        // ✅ ChainManager knows canonical head + genesis
        self.chain.lock().unwrap().status()
    }

    async fn dial(&self, addr: SocketAddr) -> Result<Connection, quinn::ConnectionError> {
        let conn = self.endpoint.connect(addr, "localhost").unwrap().await?;
        println!("[DISC] dialed {}", addr);
        Ok(conn)
    }

    async fn refresh_round(&self) {
        let peers = self.table.lock().unwrap().list();

        for p in peers {
            let addr: SocketAddr = format!("{}:{}", p.ip, p.port).parse().unwrap();
            if let Ok(conn) = self.dial(addr).await {
                if outbound_handshake(&conn, &self.local_enr.node_id, &self.local_caps)
                    .await
                    .is_ok()
                {
                    let st = self.local_status();
                    let _ = send_enveloped(
                        &conn,
                        SYNC_PROTO,
                        &MiniSyncMessage::Status(st).to_bytes(),
                    )
                    .await;
                }
            }
        }
    }
}

// ---------------- per-connection demux ----------------

async fn connection_loop(
    conn: Connection,
    local: Enr,
    table: Arc<Mutex<PeerTable>>,
    chain: Arc<Mutex<ChainManager>>,
) {
    loop {
        let Ok((_s, mut recv)) = conn.accept_bi().await else { return };

        let Ok(len) = recv.read_u32().await else { continue };
        let mut buf = vec![0u8; len as usize];
        if recv.read_exact(&mut buf).await.is_err() {
            continue;
        }

        let Some(env) = Envelope::from_bytes(&buf) else { continue };

        match env.proto.as_str() {
            DISC_PROTO => handle_discovery_msg(&conn, &local, &table, &env.data).await,
            SYNC_PROTO => handle_sync_msg(&conn, &chain, &env.data).await,
            _ => {}
        }
    }
}

// ---------------- discovery ----------------

async fn handle_discovery_msg(
    conn: &Connection,
    local: &Enr,
    table: &Arc<Mutex<PeerTable>>,
    payload: &[u8],
) {
    let Some(msg) = DiscoveryMessage::from_bytes(payload) else { return };

    match msg {
        DiscoveryMessage::Ping { from } => {
            table.lock().unwrap().insert(local, from.clone());
            let _ = send_enveloped(
                conn,
                DISC_PROTO,
                &DiscoveryMessage::Pong {
                    from: local.clone(),
                }
                .to_bytes(),
            )
            .await;
        }
        DiscoveryMessage::Nodes { from, peers } => {
            table.lock().unwrap().insert(local, from);
            table.lock().unwrap().insert_many(local, peers);
        }
        _ => {}
    }
}

// ---------------- mini-sync (fork-aware) ----------------

async fn handle_sync_msg(
    conn: &Connection,
    chain: &Arc<Mutex<ChainManager>>,
    payload: &[u8],
) {
    let Some(msg) = MiniSyncMessage::from_bytes(payload) else { return };

    match msg {
        MiniSyncMessage::Status(remote) => {
            // ✅ decide under lock, do I/O after lock is dropped
            let req_opt = {
                let mgr = chain.lock().unwrap();
                if mgr.should_request(&remote) {
                    Some(mgr.build_request(&remote))
                } else {
                    None
                }
            };

            if let Some(req) = req_opt {
                let _ = send_enveloped(conn, SYNC_PROTO, &req.to_bytes()).await;
            }
        }

        MiniSyncMessage::Headers(hs) => {
            // import_headers() is sync-only (no await), so locking is fine here
            let mut mgr = chain.lock().unwrap();
            mgr.import_headers(hs.headers);
        }

        _ => {}
    }
}

// ---------------- framed sender ----------------

async fn send_enveloped(conn: &Connection, proto: &str, payload: &[u8]) -> Result<(), ()> {
    let env = Envelope::new(proto.to_string(), payload.to_vec()).to_bytes();
    let (mut send, _) = conn.open_bi().await.map_err(|_| ())?;
    send.write_u32(env.len() as u32).await.map_err(|_| ())?;
    send.write_all(&env).await.map_err(|_| ())?;
    send.finish().await.map_err(|_| ())?;
    Ok(())
}
