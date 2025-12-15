use crate::discovery::enr::Enr;
use crate::discovery::message::DiscoveryMessage;
use crate::discovery::table::PeerTable;
use crate::session::handshake::{inbound_handshake, outbound_handshake};

use quinn::{Connection, Endpoint};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{sleep, Duration};

pub struct DiscoveryService {
    endpoint: Endpoint,
    local_enr: Enr,
    table: Arc<Mutex<PeerTable>>,
    local_caps: Vec<String>, 
}

impl DiscoveryService {
    pub fn new(endpoint: Endpoint, local_enr: Enr) -> Self {
        Self {
            endpoint,
            local_enr,
            table: Arc::new(Mutex::new(PeerTable::new(32))),
            local_caps: vec!["discv-lite/0.1".to_string()], // 👈 capability
        }
    }

    pub async fn run(self, bootstrap: Option<SocketAddr>) {
        println!(
            "[DISC] local ENR: node_id={} addr={}:{}",
            self.local_enr.node_id, self.local_enr.ip, self.local_enr.port
        );

        // -------- Inbound accept loop --------
        let accept_endpoint = self.endpoint.clone();
        let table_for_accept = self.table.clone();
        let local_for_accept = self.local_enr.clone();
        let caps_for_accept = self.local_caps.clone();

        tokio::spawn(async move {
            loop {
                if let Some(connecting) = accept_endpoint.accept().await {
                    match connecting.await {
                        Ok(conn) => {
                            let table = table_for_accept.clone();
                            let local = local_for_accept.clone();
                            let caps = caps_for_accept.clone();

                            tokio::spawn(async move {
                                match inbound_handshake(&conn, &local.node_id, &caps).await {
                                    Ok(sess) => {
                                        println!(
                                            "[SESS] inbound with {} agreed={:?}",
                                            sess.remote_node_id, sess.agreed_caps
                                        );

                                        // Capability gating
                                        if sess
                                            .agreed_caps
                                            .iter()
                                            .any(|c| c == "discv-lite/0.1")
                                        {
                                            handle_incoming(conn, local, table).await;
                                        }
                                    }
                                    Err(_) => {
                                        // incompatible peer → drop silently
                                    }
                                }
                            });
                        }
                        Err(_) => {
                            // normal QUIC close during discovery
                        }
                    }
                }
            }
        });

        // -------- Bootstrap dial --------
        if let Some(addr) = bootstrap {
            if let Ok(conn) = self.dial(addr).await {
                if let Ok(sess) = outbound_handshake(
                    &conn,
                    &self.local_enr.node_id,
                    &self.local_caps,
                )
                .await
                {
                    println!("[SESS] outbound bootstrap {:?}", sess);
                    self.bootstrap_exchange(&conn).await;
                }
            }
        }

        // -------- Periodic refresh --------
        loop {
            self.refresh_round().await;
            sleep(Duration::from_secs(3)).await;
        }
    }

    async fn dial(&self, addr: SocketAddr) -> Result<Connection, quinn::ConnectionError> {
        let connecting = self.endpoint.connect(addr, "localhost").unwrap();
        let conn = connecting.await?;
        println!("[DISC] dialed {}", addr);
        Ok(conn)
    }

    async fn bootstrap_exchange(&self, conn: &Connection) {
        let _ = send_msg(
            conn,
            &DiscoveryMessage::Ping {
                from: self.local_enr.clone(),
            },
        )
        .await;

        let _ = request_nodes(conn, &self.local_enr, self.table.clone()).await;
    }

    async fn refresh_round(&self) {
        let peers = { self.table.lock().unwrap().list() };

        for p in peers {
            let addr: SocketAddr = format!("{}:{}", p.ip, p.port).parse().unwrap();
            if let Ok(conn) = self.dial(addr).await {
                if let Ok(sess) = outbound_handshake(
                    &conn,
                    &self.local_enr.node_id,
                    &self.local_caps,
                )
                .await
                {
                    println!("[SESS] outbound {:?}", sess);

                    if sess
                        .agreed_caps
                        .iter()
                        .any(|c| c == "discv-lite/0.1")
                    {
                        let _ = send_msg(
                            &conn,
                            &DiscoveryMessage::Ping {
                                from: self.local_enr.clone(),
                            },
                        )
                        .await;

                        let _ =
                            request_nodes(&conn, &self.local_enr, self.table.clone()).await;
                    }
                }
            }
        }
    }
}

// ------------------ Discovery message handler ------------------

async fn handle_incoming(conn: Connection, local: Enr, table: Arc<Mutex<PeerTable>>) {
    loop {
        let Ok((_send, mut recv)) = conn.accept_bi().await else {
            return;
        };

        let Ok(len) = recv.read_u32().await else { continue };
        let mut data = vec![0u8; len as usize];
        if recv.read_exact(&mut data).await.is_err() {
            continue;
        }

        let Some(msg) = DiscoveryMessage::from_bytes(&data) else {
            continue;
        };

        match msg {
            DiscoveryMessage::Ping { from } => {
                println!("[DISC] PING from {}:{}", from.ip, from.port);
                table.lock().unwrap().insert(&local, from.clone());

                let _ = send_msg(
                    &conn,
                    &DiscoveryMessage::Pong {
                        from: local.clone(),
                    },
                )
                .await;
            }
            DiscoveryMessage::Pong { from } => {
                println!("[DISC] PONG from {}:{}", from.ip, from.port);
                table.lock().unwrap().insert(&local, from);
            }
            DiscoveryMessage::FindNodes { from } => {
                println!("[DISC] FIND_NODES from {}:{}", from.ip, from.port);
                table.lock().unwrap().insert(&local, from.clone());

                let peers = table.lock().unwrap().list();
                let _ = send_msg(
                    &conn,
                    &DiscoveryMessage::Nodes {
                        from: local.clone(),
                        peers,
                    },
                )
                .await;
            }
            DiscoveryMessage::Nodes { from, peers } => {
                println!(
                    "[DISC] NODES from {}:{} ({} peers)",
                    from.ip,
                    from.port,
                    peers.len()
                );
                table.lock().unwrap().insert(&local, from);
                table.lock().unwrap().insert_many(&local, peers);
            }
        }
    }
}

// ------------------ Framing helpers ------------------

async fn send_msg(conn: &Connection, msg: &DiscoveryMessage) -> Result<(), ()> {
    let payload = msg.to_bytes();
    let (mut send, _) = conn.open_bi().await.map_err(|_| ())?;

    send.write_u32(payload.len() as u32).await.map_err(|_| ())?;
    send.write_all(&payload).await.map_err(|_| ())?;
    send.finish().await.map_err(|_| ())?;
    Ok(())
}

async fn request_nodes(
    conn: &Connection,
    local: &Enr,
    table: Arc<Mutex<PeerTable>>,
) -> Result<Option<Vec<Enr>>, ()> {
    send_msg(
        conn,
        &DiscoveryMessage::FindNodes {
            from: local.clone(),
        },
    )
    .await?;

    Ok(Some(table.lock().unwrap().list()))
}
