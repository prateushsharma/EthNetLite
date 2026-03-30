use crate::protocol::mini_sync::header::Header;
use rand::{thread_rng, RngCore};

#[derive(Debug, Clone)]
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

    pub fn contains_hash(&self, hash: &str) -> bool {
        self.headers.iter().any(|h| h.hash == hash)
    }

    pub fn prefix_until(&self, hash: &str) -> Option<Chain> {
        let idx = self.headers.iter().position(|h| h.hash == hash)?;
        Some(Chain {
            headers: self.headers[..=idx].to_vec(),
        })
    }

    pub fn can_append_header(&self, header: &Header) -> bool {
        header.number == self.height() + 1 && header.parent_hash == self.head_hash()
    }

    pub fn append_header(&mut self, header: Header) -> bool {
        if self.can_append_header(&header) {
            self.headers.push(header);
            true
        } else {
            false
        }
    }

    pub fn append_linear(&mut self, headers: Vec<Header>) {
        for h in headers {
            if !self.append_header(h) {
                break;
            }
        }
    }

    pub fn produce_header(&mut self) {
        let parent = self.head().clone();

        let mut rng = thread_rng();
        let hi = rng.next_u64() as u128;
        let lo = rng.next_u64() as u128;
        let hash = format!("0x{:032x}", (hi << 64) | lo);

        let h = Header {
            parent_hash: parent.hash,
            hash,
            number: parent.number + 1,
        };

        self.headers.push(h);
    }
}