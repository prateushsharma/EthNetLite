use rand::seq::IteratorRandom;
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct Sampler {
    requested: HashSet<u64>,
    received: HashSet<u64>,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            requested: HashSet::new(),
            received: HashSet::new(),
        }
    }

    pub fn choose_random_indices(&mut self, total_chunks: u64, count: usize) -> Vec<u64> {
        let mut rng = rand::thread_rng();

        let picks: Vec<u64> = (0..total_chunks)
            .choose_multiple(&mut rng, count.min(total_chunks as usize));

        for idx in &picks {
            self.requested.insert(*idx);
        }

        picks
    }

    pub fn mark_received(&mut self, index: u64) {
        self.received.insert(index);
    }

    pub fn requested_count(&self) -> usize {
        self.requested.len()
    }

    pub fn received_count(&self) -> usize {
        self.received.len()
    }

    pub fn confidence(&self) -> f64 {
        if self.requested.is_empty() {
            0.0
        } else {
            self.received.len() as f64 / self.requested.len() as f64
        }
    }
}