use std::collections::HashMap;

use crate::protocol::das::types::{ChunkIndex, DataId, DataSet};

#[derive(Debug, Default)]
pub struct DataStore {
    datasets: HashMap<DataId, DataSet>,
}

impl DataStore {
    pub fn new() -> Self {
        Self {
            datasets: HashMap::new(),
        }
    }

    pub fn insert(&mut self, dataset: DataSet) {
        self.datasets.insert(dataset.id.clone(), dataset);
    }

    pub fn has(&self, data_id: &str) -> bool {
        self.datasets.contains_key(data_id)
    }

    pub fn total_chunks(&self, data_id: &str) -> Option<u64> {
        self.datasets.get(data_id).map(|d| d.total_chunks())
    }

    pub fn get_chunk(&self, data_id: &str, index: ChunkIndex) -> Option<Vec<u8>> {
        self.datasets.get(data_id).and_then(|d| d.chunk(index))
    }

    pub fn list_ids(&self) -> Vec<String> {
        self.datasets.keys().cloned().collect()
    }
}