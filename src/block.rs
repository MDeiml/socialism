use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct Block {
    start: u64,
    end: u64,
}

impl Block {
    pub fn intersects(&self, other: &Block) -> bool {
        self.start < other.end && self.end > other.start
    }
}

impl<'de> Deserialize<'de> for Block {
    fn deserialize<D>(deserializer: D) -> Result<Block, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct DBlock {
            start: u64,
            end: u64,
        }
        let block: DBlock = Deserialize::deserialize(deserializer)?;
        if block.start < block.end {
            Ok(Block {
                start: block.start,
                end: block.end,
            })
        } else {
            Err(serde::de::Error::custom("Block must have positive length"))
        }
    }
}
