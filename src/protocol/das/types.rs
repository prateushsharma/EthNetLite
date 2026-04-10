pub type DataId = String;
pub type ChunkIndex = u64;

#[derive(Debug, Clone)]
pub struct DataSet {
    pub id: DataId,
    pub total_size: usize,
    pub chunk_size: usize,
    pub chunks: Vec<Vec<u8>>,
}

impl DataSet {
    pub fn from_bytes(id: DataId, bytes: Vec<u8>, chunk_size: usize) -> Self {
        let total_size = bytes.len();
        let mut chunks = Vec::new();

        for chunk in bytes.chunks(chunk_size) {
            chunks.push(chunk.to_vec());
        }

        Self {
            id,
            total_size,
            chunk_size,
            chunks,
        }
    }

    pub fn total_chunks(&self) -> u64 {
        self.chunks.len() as u64
    }

    pub fn chunk(&self, index: ChunkIndex) -> Option<Vec<u8>> {
        self.chunks.get(index as usize).cloned()
    }
}