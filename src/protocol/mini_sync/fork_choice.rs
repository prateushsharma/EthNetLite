use crate::protocol::mini_sync::chain::Chain;

#[derive(Debug, Clone)]
pub enum ForkChoiceRule {
    LongestChain,
    
}

pub fn choose<'a>(
    rule: &ForkChoiceRule,
    candidates: &'a [Chain],
) -> Option<&'a Chain> {
    match rule {
        ForkChoiceRule::LongestChain => {
            candidates.iter().max_by_key(|c| c.height())
        }
    }
}
