use crate::models::block::Block;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
//`Blockchain` A structure that represents the blockchain
pub struct Blockchain {
    // Storage for the Blocks
    pub chain: Vec<Block>,
    //Min amt of work required to validate
    pub difficulty: usize,
}

impl Blockchain {
    pub fn new(difficulty: usize) -> Self {
        let genesis_block = Block::new(0, String::from("0")); // Genesis block has index 0 and empty previous hash
        let mut blockchain = Blockchain {
            chain: vec![genesis_block],
            difficulty,
        };
        blockchain.mine_block(0); // Mine the genesis block
        blockchain
    }

    pub fn add_block(&mut self) {
        let previous_hash = self.chain.last().unwrap().hash.clone();
        let index = self.chain.len() as u64;
        let mut new_block = Block::new(index, previous_hash);
        new_block.mine(self.difficulty); // Mine the new block
        self.chain.push(new_block);
    }

    fn mine_block(&mut self, index: u64) {
        if let Some(block) = self.chain.get_mut(index as usize) {
            block.mine(self.difficulty);
        }
    }

    pub fn compare<'a>(chain1: &'a Blockchain, chain2: &'a Blockchain) -> &'a Blockchain {
        if chain1.chain.len() > chain2.chain.len() {
            return chain1;
        }

        chain2
    }
}
