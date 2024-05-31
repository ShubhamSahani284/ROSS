use futures::channel::oneshot;
use serde::{Deserialize, Serialize};

use crate::models::blockchain::Blockchain;

pub mod client;
pub mod node;

#[derive(Debug)]
pub struct NodeState {
    pub blockchain: Blockchain,
    pub ready: bool,
}

impl NodeState {
    pub fn new(blockchain: Blockchain) -> NodeState {
        NodeState {
            blockchain,
            ready: false,
        }
    }
}

pub enum Command {
    RequestInit {
        sender: oneshot::Sender<Result<Option<Blockchain>, ()>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct InitRequest(String);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitResponse(Option<Blockchain>);
