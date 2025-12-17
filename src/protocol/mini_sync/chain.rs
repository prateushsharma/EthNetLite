use crate::protocol::mini_sync::header::Header;
use rand::{thread_rng, Rng};

#[derive(Debug)]
pub struct Chain {
    pub headers: Vec<Header>,
}

impl Chain {
    pub fn new(genesis_hash: String) -> Self {
        let genesis = Header {
            parent_hash: "0x00".to_string(),
            hash: genesis_hash,
            number: 0,
        };
        Self { headers: vec![genesis] }
    }

    pub fn head(&self) -> &Header {
        self.headers.last().unwrap()
    }

    pub fn height(&self) -> u64 {
        self.head().number
    }

    pub fn head_hash(&self) -> String {
        self.head().hash.clone()
    }

    // MVP fork-choice: append if it extends current head by +1
    pub fn append_linear(&mut self, headers: Vec<Header>) {
        for h in headers {
            if h.number == self.height() + 1 && h.parent_hash == self.head_hash() {
                self.headers.push(h);
            }
        }
    }
    pub fn produce_header(&mut self) {
        let parent = self.head().clone();

        let hash = format!(
            "0x{:x}",
            thread_rng().gen::<u128>()
        );

        let h = Header {
            parent_hash: parent.hash,
            hash,
            number: parent.number + 1,
        };

        self.headers.push(h);
    }
}
