use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// `Block`, A struct that represents a block in a Blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    // The index in which the current block is stored.
    pub index: u64,
    // The time the current block is created.
    pub timestamp: u64,
    // The block's proof of work.
    pub proof_of_work: u64,
    // The previous block hash.
    pub previous_hash: String,
    // The current block hash.
    pub hash: String,
}

impl Block {
    pub fn new(index: u64, previous_hash: String) -> Self {
        let timestamp = chrono::Utc::now().timestamp_millis() as u64;
        let proof_of_work = 0;
        let hash = String::new(); // Initial value, will be calculated later

        Block {
            index,
            timestamp,
            proof_of_work,
            previous_hash,
            hash,
        }
    }

    pub fn calculate_hash(&mut self) {
        let mut hasher = Sha256::new();
        hasher.update(format!(
            "{}{}{}{}",
            self.index, self.timestamp, self.proof_of_work, self.previous_hash
        ));
        let result = hasher.finalize();
        self.hash = format!("{:x}", result);
    }

    pub fn mine(&mut self, difficulty: usize) {
        loop {
            self.proof_of_work += 1;
            self.calculate_hash();
            if self.hash.starts_with(&"0".repeat(difficulty)) {
                break;
            }
        }
    }
}
