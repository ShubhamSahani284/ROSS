mod models;
mod p2p;

use futures::channel::mpsc;
use models::blockchain::Blockchain;
use p2p::NodeState;
use std::{error::Error, fs, path::Path, sync::Arc};

use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let blockchain = load_blockchain();
    let (sender, receiver) = mpsc::channel(0);

    let node_state = Arc::new(Mutex::new(NodeState::new(blockchain)));

    let node = p2p::node::Node::new(receiver, Arc::clone(&node_state)).await?;
    let mut client = p2p::client::Client::new(sender, Arc::clone(&node_state));

    tokio::task::spawn(node.start(None));

    {
        let state = node_state.lock().await;
        if state.blockchain.chain.is_empty() {
            drop(state); // drop the mutex guard since this can be a long operation

            let res = client.request_init().await;

            let mut state = node_state.lock().await;

            let blockchain = match res {
                Some(blockchain) => {
                    println!("downloaded chain {blockchain:?}");
                    blockchain
                }
                None => {
                    println!("Could not init, generating genesis block");
                    Blockchain::new(0)
                }
            };
            state.blockchain = blockchain;
        }
    }

    loop {}

    // TODO
    // listen for chain events through gossip sub

    Ok(())
}

fn load_blockchain() -> Blockchain {
    let path = Path::new("blockchain.json");
    if path.exists() {
        let contents = fs::read_to_string(path).expect("Failed to read blockchain file");
        serde_json::from_str(&contents).expect("Failed to deserialize blockchain")
    } else {
        Blockchain {
            chain: Vec::new(),
            difficulty: 0,
        }
    }
}

fn save_blockchain(blockchain: &Blockchain) {
    let contents = serde_json::to_string(blockchain).expect("Failed to serialize blockchain");
    fs::write("blockchain.json", contents).expect("Failed to write blockchain file");
}
