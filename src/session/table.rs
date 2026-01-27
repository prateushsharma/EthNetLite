use quinn::Connection;
use std::collections::HashMap;
use std::time::Instant;

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
imple SessionTable {
    pub fn new() -> Self {
        Self { 
            sessions: HashMap:::new(),
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
}
