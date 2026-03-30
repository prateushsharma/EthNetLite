use std::collections::HashMap;

use crate::protocol::mini_sync::{
    chain::Chain,
    fork_choice::{choose, ForkChoiceRule},
    message::{MiniSyncMessage, RequestHeaders, Status},
};

#[derive(Debug)]
pub struct ChainManager {
    chains: HashMap<String, Chain>, // head_hash -> chain
    canonical_head: String,
    rule: ForkChoiceRule,
}

impl ChainManager {
    pub fn new(genesis_hash: String) -> Self {
        let chain = Chain::new(genesis_hash);
        let head = chain.head_hash();

        let mut chains = HashMap::new();
        chains.insert(head.clone(), chain);

        Self {
            chains,
            canonical_head: head,
            rule: ForkChoiceRule::LongestChain,
        }
    }

    // -------- Canonical accessors --------

    pub fn canonical(&self) -> &Chain {
        self.chains
            .get(&self.canonical_head)
            .expect("canonical exists")
    }

    pub fn canonical_height(&self) -> u64 {
        self.canonical().height()
    }

    pub fn canonical_head_hash(&self) -> String {
        self.canonical().head_hash()
    }

    pub fn genesis_hash(&self) -> String {
        // genesis header is always headers[0]
        self.canonical().headers[0].hash.clone()
    }

    pub fn status(&self) -> Status {
        Status {
            genesis_hash: self.genesis_hash(),
            head_hash: self.canonical_head_hash(),
            head_number: self.canonical_height(),
        }
    }

    /// Clone the canonical chain so we can extend it as a candidate fork.
    pub fn canonical_clone(&self) -> Chain {
        self.canonical().clone()
    }

    // -------- Fork mgmt --------

    pub fn insert_chain(&mut self, chain: Chain) {
        let head = chain.head_hash();
        self.chains.insert(head, chain);
        self.recompute();
    }

    pub fn recompute(&mut self) {
        let all: Vec<Chain> = self.chains.values().cloned().collect();
        if let Some(best) = choose(&self.rule, &all) {
            let new_head = best.head_hash();
            if new_head != self.canonical_head {
                println!(
                    "[FORK] canonical switch {} -> {} (height={})",
                    self.canonical_head,
                    new_head,
                    best.height()
                );
                self.canonical_head = new_head;
            }
        }
    }

    // -------- Sync helpers (7C integration) --------

    pub fn should_request(&self, remote: &Status) -> bool {
        // same network?
        if remote.genesis_hash != self.genesis_hash() {
            return false;
        }
        remote.head_number > self.canonical_height()
    }

    pub fn build_request(&self, remote: &Status) -> MiniSyncMessage {
        let local_h = self.canonical_height();
        MiniSyncMessage::RequestHeaders(RequestHeaders {
            start: local_h + 1,
            count: remote.head_number - local_h,
        })
    }

    pub fn import_headers(&mut self, headers: Vec<crate::protocol::mini_sync::header::Header>) {
    if headers.is_empty() {
        return;
    }

    let before = self.canonical_height();

    for header in headers {
        let mut candidate_chain = None;

        for chain in self.chains.values() {
            if chain.contains_hash(&header.parent_hash) {
                if let Some(prefix) = chain.prefix_until(&header.parent_hash) {
                    candidate_chain = Some(prefix);
                    break;
                }
            }
        }

        let Some(mut candidate) = candidate_chain else {
            println!(
                "[SYNC] dropped header {}: unknown parent {}",
                header.hash, header.parent_hash
            );
            continue;
        };

        if candidate.append_header(header.clone()) {
            self.insert_chain(candidate);
        } else {
            println!(
                "[SYNC] dropped header {}: invalid linear extension",
                header.hash
            );
        }
    }

    let after = self.canonical_height();
    if after > before {
        println!("[SYNC] advanced canonical head {} -> {}", before, after);
    }
}
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::mini_sync::header::Header;

    #[test]
    fn test_fork_switch_to_longer_branch() {
        let genesis = "0xgenesis".to_string();
        let mut mgr = ChainManager::new(genesis.clone());

        // Canonical: genesis -> a1 -> a2
        mgr.import_headers(vec![
            Header {
                parent_hash: genesis.clone(),
                hash: "0xa1".to_string(),
                number: 1,
            },
            Header {
                parent_hash: "0xa1".to_string(),
                hash: "0xa2".to_string(),
                number: 2,
            },
        ]);

        assert_eq!(mgr.canonical_head_hash(), "0xa2");
        assert_eq!(mgr.canonical_height(), 2);

        // Fork from genesis: genesis -> b1
        mgr.import_headers(vec![
            Header {
                parent_hash: genesis.clone(),
                hash: "0xb1".to_string(),
                number: 1,
            },
        ]);

        // Canonical should still be a2 because height 2 > 1
        assert_eq!(mgr.canonical_head_hash(), "0xa2");
        assert_eq!(mgr.canonical_height(), 2);

        // Extend fork: genesis -> b1 -> b2 -> b3
        mgr.import_headers(vec![
            Header {
                parent_hash: "0xb1".to_string(),
                hash: "0xb2".to_string(),
                number: 2,
            },
            Header {
                parent_hash: "0xb2".to_string(),
                hash: "0xb3".to_string(),
                number: 3,
            },
        ]);

        // Fork should now win
        assert_eq!(mgr.canonical_head_hash(), "0xb3");
        assert_eq!(mgr.canonical_height(), 3);
    }
}
