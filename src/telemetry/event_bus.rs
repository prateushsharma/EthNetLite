use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

/// Every observable thing that happens in the node.
/// The gRPC StreamEvents method forwards these to connected clients.
#[derive(Debug, Clone)]
pub struct NetworkEvent {
    pub event_type: String, // "canonical_switch" | "peer_join" | "peer_leave" | "fork_detected"
    pub peer_id: String,    // empty string when not peer-specific
    pub detail: String,     // human-readable description
    pub timestamp: u64,     // unix seconds
}

impl NetworkEvent {
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn canonical_switch(old: &str, new: &str, height: u64) -> Self {
        Self {
            event_type: "canonical_switch".into(),
            peer_id: String::new(),
            detail: format!("{} -> {} (height={})", old, new, height),
            timestamp: Self::now(),
        }
    }

    pub fn peer_join(peer_id: &str, caps: &[String]) -> Self {
        Self {
            event_type: "peer_join".into(),
            peer_id: peer_id.to_string(),
            detail: format!("agreed_caps={:?}", caps),
            timestamp: Self::now(),
        }
    }

    pub fn peer_leave(peer_id: &str) -> Self {
        Self {
            event_type: "peer_leave".into(),
            peer_id: peer_id.to_string(),
            detail: "connection closed".into(),
            timestamp: Self::now(),
        }
    }

    pub fn fork_detected(old: &str, new: &str, height: u64) -> Self {
        Self {
            event_type: "fork_detected".into(),
            peer_id: String::new(),
            detail: format!("fork: {} -> {} at height {}", old, new, height),
            timestamp: Self::now(),
        }
    }
}

/// Thin Arc-cloneable wrapper around a broadcast sender.
///
/// Why broadcast and not mpsc?
/// Because multiple subscribers (gRPC streams, future metrics, loggers)
/// each need their own independent copy of every event.
/// broadcast::channel gives exactly that.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<NetworkEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        // 256-event buffer. If a slow subscriber falls behind, it gets
        // RecvError::Lagged — we skip those gracefully in the gRPC stream.
        let (tx, _) = broadcast::channel(256);
        Self { tx }
    }

    /// Publish an event to all current subscribers.
    /// Silently drops if nobody is subscribed yet.
    pub fn publish(&self, event: NetworkEvent) {
        let _ = self.tx.send(event);
    }

    /// Get a new independent receiver.
    /// Each gRPC StreamEvents connection calls this once.
    pub fn subscribe(&self) -> broadcast::Receiver<NetworkEvent> {
        self.tx.subscribe()
    }
}