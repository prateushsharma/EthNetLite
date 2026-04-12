use std::collections::HashMap;

use crate::protocol::das::{
    message::DasMessage,
    sampler::Sampler,
    store::DataStore,
    types::{DataId, DataSet},
};

#[derive(Debug)]
pub struct DasManager {
    store: DataStore,
    announced: HashMap<DataId, u64>, // data_id -> total_chunks
    samplers: HashMap<DataId, Sampler>,
}

impl DasManager {
    pub fn new() -> Self {
        Self {
            store: DataStore::new(),
            announced: HashMap::new(),
            samplers: HashMap::new(),
        }
    }

    pub fn insert_dataset(&mut self, id: DataId, bytes: Vec<u8>, chunk_size: usize) -> DasMessage {
        let dataset = DataSet::from_bytes(id.clone(), bytes, chunk_size);
        let total_chunks = dataset.total_chunks();
        self.store.insert(dataset);

        DasMessage::AnnounceData {
            data_id: id,
            total_chunks,
        }
    }

    pub fn sample_requests(&mut self, data_id: &str, count: usize) -> Vec<DasMessage> {
        let Some(total_chunks) = self.announced.get(data_id).copied() else {
            return vec![];
        };

        let sampler = self
            .samplers
            .entry(data_id.to_string())
            .or_insert_with(Sampler::new);

        sampler
            .choose_random_indices(total_chunks, count)
            .into_iter()
            .map(|index| DasMessage::RequestChunk {
                data_id: data_id.to_string(),
                index,
            })
            .collect()
    }

    pub fn handle_message(&mut self, msg: DasMessage) -> Option<DasMessage> {
        match msg {
            DasMessage::AnnounceData { data_id, total_chunks } => {
                println!(
                    "[DAS] announced dataset id={} total_chunks={}",
                    data_id, total_chunks
                );
                self.announced.insert(data_id, total_chunks);
                None
            }

            DasMessage::RequestChunk { data_id, index } => {
                let bytes = self.store.get_chunk(&data_id, index)?;

                Some(DasMessage::ChunkResponse {
                    data_id,
                    index,
                    bytes,
                })
            }

            DasMessage::ChunkResponse { data_id, index, bytes } => {
                println!(
                    "[DAS] received chunk data_id={} index={} bytes={}",
                    data_id,
                    index,
                    bytes.len()
                );

                let sampler = self
                    .samplers
                    .entry(data_id.clone())
                    .or_insert_with(Sampler::new);

                sampler.mark_received(index);

                println!(
                    "[DAS] sampling status data_id={} requested={} received={} confidence={:.2}",
                    data_id,
                    sampler.requested_count(),
                    sampler.received_count(),
                    sampler.confidence()
                );

                None
            }
        }
    }

    pub fn store(&self) -> &DataStore {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut DataStore {
        &mut self.store
    }
}