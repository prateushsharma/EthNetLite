use crate::protocol::das::{
    message::DasMessage,
    store::DataStore,
    types::{DataId, DataSet},
};

#[derive(Debug)]
pub struct DasManager {
    store: DataStore,
}

impl DasManager {
    pub fn new() -> Self {
        Self {
            store: DataStore::new(),
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

    pub fn handle_message(&mut self, msg: DasMessage) -> Option<DasMessage> {
        match msg {
            DasMessage::AnnounceData { data_id, total_chunks } => {
                println!(
                    "[DAS] announced dataset id={} total_chunks={}",
                    data_id, total_chunks
                );
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