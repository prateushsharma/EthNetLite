use crate::discovery::enr::Enr;
use crate::discovery::message::DiscoveryMessage;
use crate::discovery::table::PeerTable;

use quinn::{Connection, Endpoint};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{sleep, Duration};

pub struct DiscoveryService {
    endpoint: Endpoint,
    local_enr: Enr,
    table: Arc<Mutex<PeerTable>>,
}

impl DiscoveryService {
    pub fn new(endpoint: Endpoint, local_enr: Enr) -> Self {
        Self {
            endpoint,
            local_enr,
            table: Arc::new(Mutex::new(PeerTable::new(32))),
        }
    }

    pub async fn run(mut self, bootstrap: Option<SocketAddr>) {
        println!(
            "[DISC] local ENR: node_id={} addr={}:{}",
            self.local_enr.node_id, self.local_enr.ip, self.local_enr.port
        );

        // accept loop
        let accept_endpoint = self.endpoint.clone();
        let table_for_accept = self.table.clone();
        let local_for_accept = self.local_enr.clone();

        tokio::spawn(async move {
            loop {
                if let Some(connecting) = accept_endpoint.accept().await {
                    match connecting.await {
                        Ok(conn) => {
                            let table = table_for_accept.clone();
                            let local = local_for_accept.clone();
                            tokio::spawn(async move {
                                handle_incoming(conn, local, table).await;
                            });
                        }
                        Err(e) => {
                            eprintln!("[DISC] accept error: {e:?}");
                        }
                    }
                }
            }
        });

        // bootstrap dial once
        if let Some(addr) = bootstrap {
            if let Ok(conn) = self.dial(addr).await {
                self.bootstrap_exchange(&conn).await;
            }
        }

        // periodic refresh loop
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
        // Ping
        let _ = send_msg(conn, &DiscoveryMessage::Ping { from: self.local_enr.clone() }).await;

        // FindNodes
        if let Ok(Some(nodes)) =
            request_nodes(conn, &self.local_enr, self.table.clone()).await
        {
            println!("[DISC] bootstrap returned {} peers", nodes.len());
        }
    }

    async fn refresh_round(&self) {
        let peers = { self.table.lock().unwrap().list() };

        for p in peers {
            let addr: SocketAddr = format!("{}:{}", p.ip, p.port).parse().unwrap();
            match self.dial(addr).await {
                Ok(conn) => {
                    // Ping + FindNodes
                    let _ = send_msg(&conn, &DiscoveryMessage::Ping { from: self.local_enr.clone() }).await;
                    let _ = request_nodes(&conn, &self.local_enr, self.table.clone()).await;

                    // If we got new peers, attempt to dial them quickly (fan-out)
                    let newly = { self.table.lock().unwrap().list() };
                    for np in newly {
                        let np_addr: SocketAddr = format!("{}:{}", np.ip, np.port).parse().unwrap();
                        let _ = self.endpoint.connect(np_addr, "localhost"); // fire & forget dial (connect happens later)
                    }
                }
                Err(_) => {
                    // ignore dead peers for now (we can add pruning later)
                }
            }
        }
    }
}

async fn handle_incoming(conn: Connection, local: Enr, table: Arc<Mutex<PeerTable>>) {
    loop {
        let Ok((_send, mut recv)) = conn.accept_bi().await else {
            return;
        };

        // length-prefixed frame: u32 length + payload
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

                let _ = send_msg(&conn, &DiscoveryMessage::Pong { from: local.clone() }).await;
            }
            DiscoveryMessage::Pong { from } => {
                println!("[DISC] PONG from {}:{}", from.ip, from.port);
                table.lock().unwrap().insert(&local, from);
            }
            DiscoveryMessage::FindNodes { from } => {
                println!("[DISC] FIND_NODES from {}:{}", from.ip, from.port);
                table.lock().unwrap().insert(&local, from.clone());

                let peers = table.lock().unwrap().list();
                let _ = send_msg(&conn, &DiscoveryMessage::Nodes { from: local.clone(), peers }).await;
            }
            DiscoveryMessage::Nodes { from, peers } => {
                println!("[DISC] NODES from {}:{} ({} peers)", from.ip, from.port, peers.len());
                table.lock().unwrap().insert(&local, from);
                let added = table.lock().unwrap().insert_many(&local, peers);
                if !added.is_empty() {
                    println!("[DISC] added {} new peers", added.len());
                }
            }
        }
    }
}

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
    send_msg(conn, &DiscoveryMessage::FindNodes { from: local.clone() }).await?;

    // We don't wait synchronously for a response on the same stream; incoming handler will process Nodes.
    // For MVP, return current table snapshot.
    Ok(Some(table.lock().unwrap().list()))
}
