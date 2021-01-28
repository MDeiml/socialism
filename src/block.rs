use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Block {
    start: u64,
    end: u64,
}

impl Block {
    pub fn intersects(&self, other: &Block) -> bool {
        self.start < other.end && self.end > other.start
    }
}
