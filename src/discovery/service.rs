
use crate::enr::EnrRecord;
use crate::discovery::message::{DiscoveryMessage, WireEnr};
use crate::discovery::table::PeerTable;

use crate::protocol::envelope::Envelope;
use crate::protocol::mini_sync::manager::ChainManager;
use crate::protocol::mini_sync::message::{MiniSyncMessage, Status};
use crate::protocol::mini_sync::producer::start_header_producer;

use crate::session::handshake::{inbound_handshake, outbound_handshake};
use crate::session::table::SessionTable;

use crate::telemetry::event_bus::{EventBus, NetworkEvent};

use quinn::{Connection, Endpoint};
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{sleep, Duration};
use std::convert::TryFrom;

const DISC_PROTO: &str = "discv-lite/0.1";
const SYNC_PROTO: &str = "mini-sync/0.1";

// ─────────────────────────────────────────────────────────────────────────────

fn wire_enr_get(enr: &WireEnr, key: &[u8]) -> Option<Vec<u8>> {
    enr.pairs
        .iter()
        .find(|(k, _)| k.as_slice() == key)
        .map(|(_, v)| v.clone())
}

fn wire_enr_ip(enr: &WireEnr) -> Option<String> {
    wire_enr_get(enr, b"ip").and_then(|v| String::from_utf8(v.clone()).ok())
}

fn wire_enr_quic_port(enr: &WireEnr) -> Option<u16> {
    wire_enr_get(enr, b"quic").and_then(|v| {
        if v.len() == 2 {
            Some(u16::from_be_bytes([v[0], v[1]]))
        } else {
            None
        }
    })
}

pub struct DiscoveryService {
    endpoint: Endpoint,
    local_enr: EnrRecord,
    table: Arc<Mutex<PeerTable>>,
    chain: Arc<Mutex<ChainManager>>,
    local_caps: Vec<String>,
    sessions: Arc<Mutex<SessionTable>>,
    event_bus: EventBus,
    canonical_switches: Arc<AtomicU64>,
}

impl DiscoveryService {
    /// Original constructor — kept for backwards compat.
    pub fn new(endpoint: Endpoint, local_enr: EnrRecord) -> Self {
        let genesis = "0xgenesis".to_string();
        Self {
            endpoint,
            local_enr,
            table: Arc::new(Mutex::new(PeerTable::new(32))),
            chain: Arc::new(Mutex::new(ChainManager::new(genesis))),
            local_caps: vec![DISC_PROTO.to_string(), SYNC_PROTO.to_string()],
            sessions: Arc::new(Mutex::new(SessionTable::new())),
            event_bus: EventBus::new(),
            canonical_switches: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Module 9A constructor — accepts pre-built Arc refs so that
    /// gRPC server and P2P loops share the exact same state.
    pub fn new_with_state(
        endpoint: Endpoint,
        local_enr: EnrRecord,
        chain: Arc<Mutex<ChainManager>>,
        sessions: Arc<Mutex<SessionTable>>,
        event_bus: EventBus,
        canonical_switches: Arc<AtomicU64>,
    ) -> Self {
        Self {
            endpoint,
            local_enr,
            table: Arc::new(Mutex::new(PeerTable::new(32))),
            chain,
            local_caps: vec![DISC_PROTO.to_string(), SYNC_PROTO.to_string()],
            sessions,
            event_bus,
            canonical_switches,
        }
    }

    pub async fn run(self, bootstrap: Option<SocketAddr>) {
        if self.local_enr.quic_port().unwrap_or(0) == 9001 {
            start_header_producer(self.chain.clone(), 2).await;
            println!("[MINE] header producer enabled");
        }

        println!(
            "[DISC] local ENR: node_id={} addr={}:{}",
            self.local_enr.node_id().unwrap_or_default(),
            self.local_enr.ip().unwrap_or_default(),
            self.local_enr.quic_port().unwrap_or(0)
        );

        // ── Inbound accept loop ───────────────────────────────────────────────
        let ep                = self.endpoint.clone();
        let table             = self.table.clone();
        let chain             = self.chain.clone();
        let local             = self.local_enr.clone();
        let caps              = self.local_caps.clone();
        let sessions          = self.sessions.clone();
        let event_bus         = self.event_bus.clone();
        let canonical_switches = self.canonical_switches.clone();

        tokio::spawn(async move {
            loop {
                if let Some(connecting) = ep.accept().await {
                    if let Ok(conn) = connecting.await {
                        let table             = table.clone();
                        let chain             = chain.clone();
                        let local             = local.clone();
                        let caps              = caps.clone();
                        let sessions          = sessions.clone();
                        let event_bus         = event_bus.clone();
                        let canonical_switches = canonical_switches.clone();

                        tokio::spawn(async move {
                            let sess = match inbound_handshake(&conn, &local.node_id().unwrap_or_default(), &caps).await {
                                Ok(s)  => s,
                                Err(_) => return,
                            };

                            println!(
                                "[SESS] inbound {} agreed={:?}",
                                sess.remote_node_id, sess.agreed_caps
                            );

                            let peer_id = sess.remote_node_id.clone();

                            // ── Dedup: one session per peer ───────────────
                            {
                                let mut st = sessions.lock().unwrap();
                                if st.has(&peer_id) {
                                    println!("[SESS] duplicate peer {}, closing", peer_id);
                                    conn.close(0u32.into(), b"duplicate-session");
                                    return;
                                }
                                st.insert(crate::session::table::SessionEntry {
                                    peer_id: peer_id.clone(),
                                    conn: conn.clone(),
                                    agreed_caps: sess.agreed_caps.clone(),
                                    last_active: std::time::Instant::now(),
                                });
                            }

                            // ── Publish peer_join ─────────────────────────
                            event_bus.publish(NetworkEvent::peer_join(
                                &peer_id,
                                &sess.agreed_caps,
                            ));

                            connection_loop(
                                conn, local, table, chain,
                                sessions, event_bus, canonical_switches, peer_id,
                            ).await;
                        });
                    }
                }
            }
        });

        // ── Heartbeat + idle eviction ─────────────────────────────────────────
        let sessions_hb   = self.sessions.clone();
        let local_hb      = self.local_enr.clone();
        let event_bus_hb  = self.event_bus.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(
                    crate::session::table::HEARTBEAT_SECS,
                )).await;

                let conns = {
                    let st = sessions_hb.lock().unwrap();
                    st.snapshot_conns()
                };

                for (_peer_id, conn) in conns {
                    let _ = send_enveloped(
                        &conn, DISC_PROTO,
                        &DiscoveryMessage::Ping { from: (&local_hb).into() }.to_bytes(),
                    ).await;
                }

                let idle = {
                    let st = sessions_hb.lock().unwrap();
                    st.idle_peers()
                };

                if !idle.is_empty() {
                    let mut st = sessions_hb.lock().unwrap();
                    for peer in idle {
                        println!("[SESS] evict idle peer {}", peer);
                        st.remove(&peer);
                        event_bus_hb.publish(NetworkEvent::peer_leave(&peer));
                    }
                }
            }
        });

        // ── Bootstrap ────────────────────────────────────────────────────────
        if let Some(addr) = bootstrap {
            if let Ok(conn) = self.dial(addr).await {
                if let Ok(sess) =
                    outbound_handshake(&conn, &self.local_enr.node_id().unwrap_or_default(), &self.local_caps).await
                {
                    println!("[SESS] outbound bootstrap {:?}", sess);

                    send_enveloped(
                        &conn, DISC_PROTO,
                        &DiscoveryMessage::Ping { from: (&self.local_enr).into() }.to_bytes(),
                    ).await.ok();

                    let st = self.local_status();
                    send_enveloped(&conn, SYNC_PROTO, &MiniSyncMessage::Status(st).to_bytes())
                        .await.ok();
                }
            }
        }

        // ── Refresh loop ─────────────────────────────────────────────────────
        loop {
            self.refresh_round().await;
            sleep(Duration::from_secs(3)).await;
        }
    }

    fn local_status(&self) -> Status {
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
           let Some(ip) = wire_enr_ip(&p) else { continue };
let Some(quic_port) = wire_enr_quic_port(&p) else { continue };
let Ok(addr) = format!("{}:{}", ip, quic_port).parse::<SocketAddr>() else { continue };
            if let Ok(conn) = self.dial(addr).await {
                if outbound_handshake(&conn, &self.local_enr.node_id().unwrap_or_default(), &self.local_caps)
                    .await.is_ok()
                {
                    let st = self.local_status();
                    let _ = send_enveloped(
                        &conn, SYNC_PROTO, &MiniSyncMessage::Status(st).to_bytes(),
                    ).await;
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-connection demux loop
// ─────────────────────────────────────────────────────────────────────────────

async fn connection_loop(
    conn: Connection,
    local: EnrRecord,
    table: Arc<Mutex<PeerTable>>,
    chain: Arc<Mutex<ChainManager>>,
    sessions: Arc<Mutex<SessionTable>>,
    event_bus: EventBus,
    canonical_switches: Arc<AtomicU64>,
    peer_id: String,
) {
    loop {
        let Ok((_s, mut recv)) = conn.accept_bi().await else {
            // Connection closed → remove session, publish peer_leave
            sessions.lock().unwrap().remove(&peer_id);
            event_bus.publish(NetworkEvent::peer_leave(&peer_id));
            println!("[SESS] removed {}", peer_id);
            return;
        };

        let Ok(len) = recv.read_u32().await else { continue };
        let mut buf = vec![0u8; len as usize];
        if recv.read_exact(&mut buf).await.is_err() { continue; }

        let Some(env) = Envelope::from_bytes(&buf) else { continue };
        sessions.lock().unwrap().touch(&peer_id);

        match env.proto.as_str() {
            DISC_PROTO => handle_discovery_msg(&conn, &local, &table, &env.data).await,
            SYNC_PROTO => {
                handle_sync_msg(&conn, &chain, &env.data, &event_bus, &canonical_switches).await
            }
            _ => {}
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Discovery handler
// ─────────────────────────────────────────────────────────────────────────────

async fn handle_discovery_msg(
    conn: &Connection,
    local: &EnrRecord,
    table: &Arc<Mutex<PeerTable>>,
    payload: &[u8],
) {
    let Some(msg) = DiscoveryMessage::from_bytes(payload) else { return };

    match msg {
        DiscoveryMessage::Ping { from } => {
            let Ok(remote_enr) = EnrRecord::try_from(from.clone()) else {
                println!("[DISC] rejected peer: malformed ENR");
                return;
            };

            if !remote_enr.verify_self() {
                println!(
                    "[DISC] rejected peer: invalid ENR signature for node_id={}",
                    remote_enr.node_id().unwrap_or_default()
                );
                return;
            }

            let local_wire = WireEnr::from(local);
            table.lock().unwrap().insert(&local_wire, from);

            let _ = send_enveloped(
                conn,
                DISC_PROTO,
                &DiscoveryMessage::Pong { from: local_wire }.to_bytes(),
            ).await;
        }

        DiscoveryMessage::Nodes { from, peers } => {
            let local_wire = WireEnr::from(local);

            let Ok(remote_enr) = EnrRecord::try_from(from.clone()) else {
                println!("[DISC] rejected Nodes sender: malformed ENR");
                return;
            };

            if !remote_enr.verify_self() {
                println!(
                    "[DISC] rejected Nodes sender: invalid ENR signature for node_id={}",
                    remote_enr.node_id().unwrap_or_default()
                );
                return;
            }

            table.lock().unwrap().insert(&local_wire, from);

            let mut verified_peers = Vec::new();
            for peer in peers {
                let Ok(peer_enr) = EnrRecord::try_from(peer.clone()) else {
                    println!("[DISC] skipped peer in Nodes: malformed ENR");
                    continue;
                };

                if !peer_enr.verify_self() {
                    println!(
                        "[DISC] skipped peer in Nodes: invalid ENR signature for node_id={}",
                        peer_enr.node_id().unwrap_or_default()
                    );
                    continue;
                }

                verified_peers.push(peer);
            }

            table.lock().unwrap().insert_many(&local_wire, verified_peers);
        }

        _ => {}
    }
}
// ─────────────────────────────────────────────────────────────────────────────
// Sync handler — publishes canonical_switch events
// ─────────────────────────────────────────────────────────────────────────────

async fn handle_sync_msg(
    conn: &Connection,
    chain: &Arc<Mutex<ChainManager>>,
    payload: &[u8],
    event_bus: &EventBus,
    canonical_switches: &Arc<AtomicU64>,
) {
    let Some(msg) = MiniSyncMessage::from_bytes(payload) else { return };

    match msg {
        MiniSyncMessage::Status(remote) => {
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
            let (reorg, height_after) = {
                let mut mgr = chain.lock().unwrap();
                let reorg = mgr.import_headers(hs.headers);
                let height_after = mgr.canonical_height();
                (reorg, height_after)
            };

            if let Some(info) = reorg {
                canonical_switches.fetch_add(1, Ordering::Relaxed);

                println!(
                    "[REORG] fork_point={} depth={} removed={:?} added={:?}",
                    info.fork_point, info.depth, info.removed, info.added
                );

                event_bus.publish(NetworkEvent::canonical_switch(
                    &info.old_head,
                    &info.new_head,
                    height_after,
                ));
            }
        }

        _ => {}
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Framed sender helper
// ─────────────────────────────────────────────────────────────────────────────

async fn send_enveloped(conn: &Connection, proto: &str, payload: &[u8]) -> Result<(), ()> {
    let env = Envelope::new(proto.to_string(), payload.to_vec()).to_bytes();
    let (mut send, _) = conn.open_bi().await.map_err(|_| ())?;
    send.write_u32(env.len() as u32).await.map_err(|_| ())?;
    send.write_all(&env).await.map_err(|_| ())?;
    send.finish().await.map_err(|_| ())?;
    Ok(())
}