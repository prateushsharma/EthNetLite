use std::collections::HashMap;

use crate::protocol::mini_sync::{
    chain::Chain,
    fork_choice::{choose, ForkChoiceRule},
};

#[derive(Debug)]
pub struct ChainManager {
    pub chains: HashMap<String, Chain>, // head_hash → chain
    pub canonical_head: String,
    pub rule: ForkChoiceRule,
}

impl ChainManager {
    pub fn new(genesis_hash: String) -> Self {
        let chain = Chain::new(genesis_hash.clone());
        let head = chain.head_hash();

        let mut chains = HashMap::new();
        chains.insert(head.clone(), chain);

        Self {
            chains,
            canonical_head: head,
            rule: ForkChoiceRule::LongestChain,
        }
    }

    pub fn canonical(&self) -> &Chain {
        self.chains.get(&self.canonical_head).unwrap()
    }

    pub fn insert_chain(&mut self, chain: Chain) {
        let head = chain.head_hash();
        self.chains.insert(head, chain);
        self.recompute();
    }

    pub fn recompute(&mut self) {
        let all: Vec<_> = self.chains.values().cloned().collect();

        if let Some(best) = choose(&self.rule, &all) {
            self.canonical_head = best.head_hash();
        }
    }
}
