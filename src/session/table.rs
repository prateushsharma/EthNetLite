use quinn::Connection;
use std::collections::HashMap;
use std::time::Instant;

pub const HEARTBEAT_SECS: u64 = 3;
pub const IDLE_TIMEOUT_SECS: u64 = 12;

#[derive(Debug)]
pub struct SessionEntry {
    pub peer_id: String,
    pub conn: Connection,
    pub agreed_caps: Vec<String>,
    pub last_active: Instant,
}

#[derive(Debug)]
pub struct SessionTable {
    sessions: HashMap<String, SessionEntry>, // peer_id - > session
}
impl SessionTable {
    pub fn new() -> Self {
        Self { 
            sessions: HashMap::new(),
        }
    }

    // returns true is a session for this peer already exists
    pub fn has(&self, peer_id: &str) -> bool {
        self.sessions.contains_key(peer_id)
    }

    // insert ( or replace) a session entry
    pub fn insert(&mut self, entry: SessionEntry) {
        self.sessions.insert(entry.peer_id.clone(), entry);
    }

    // update liveness timestamp
    pub fn touch(&mut self, peer_id: &str) {
        if let Some(s) = self.sessions.get_mut(peer_id) {
            s.last_active = Instant::now();
        }
    }

    /// Remove a session (disconnect / prune)
    pub fn remove(&mut self, peer_id: &str) {
        self.sessions.remove(peer_id);
    }

        /// List all connected peers
    pub fn list_peers(&self) -> Vec<String> {
        self.sessions.keys().cloned().collect()
    }

    pub fn idle_peers(&self) -> Vec<String> {
    let now = Instant::now();
    self.sessions
        .iter()
        .filter_map(|(peer_id, s)| {
            if now.duration_since(s.last_active).as_secs() >= IDLE_TIMEOUT_SECS {
                Some(peer_id.clone())
            } else {
                None
            }
        })
        .collect()
}
pub fn snapshot_conns(&self) -> Vec<(String, Connection)> {
    self.sessions
        .iter()
        .map(|(peer_id, s)| (peer_id.clone(), s.conn.clone()))
        .collect()
}



}
