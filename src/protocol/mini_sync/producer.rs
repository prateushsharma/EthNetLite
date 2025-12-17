use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

use crate::protocol::mini_sync::chain::Chain;

pub async fn start_header_producer(
    chain: Arc<Mutex<Chain>>,
    interval_secs: u64,
) {
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(interval_secs)).await;

            let mut c = chain.lock().unwrap();
            let before = c.height();
            c.produce_header();
            let after = c.height();

            println!(
                "[MINE] produced header {} → {}",
                before, after
            );
        }
    });
}
