use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use crate::protocol::mini_sync::manager::ChainManager;

pub async fn start_header_producer(mgr: Arc<Mutex<ChainManager>>, interval_secs: u64) {
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(interval_secs)).await;

            let mut m = mgr.lock().unwrap();

            let before = m.canonical_height();
            let mut next = m.canonical_clone();
            next.produce_header();
            m.insert_chain(next);
            let after = m.canonical_height();

            println!("[MINE] produced header {} → {}", before, after);
        }
    });
}
