#[derive(Debug, Clone)]
pub struct PeerSession {
    pub remote_node_id: String,
    pub agreed_caps: Vec<String>,
}
