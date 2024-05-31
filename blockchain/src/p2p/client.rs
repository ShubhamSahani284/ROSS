use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

use futures::{
    channel::{
        mpsc::{self, SendError},
        oneshot,
    },
    SinkExt,
};

use crate::models::blockchain::Blockchain;

use async_std;

use super::{Command, NodeState};

#[derive(Clone)]
pub(crate) struct Client {
    sender: mpsc::Sender<Command>,
    node_state: Arc<Mutex<NodeState>>,
}

impl Client {
    pub fn new(sender: mpsc::Sender<Command>, node_state: Arc<Mutex<NodeState>>) -> Client {
        Client { sender, node_state }
    }

    async fn send_if_ready(&mut self, command: Command) -> Result<(), SendError> {
        loop {
            let state = self.node_state.lock().await;
            if state.ready == true {
                let res = self.sender.send(command).await;
                return res;
            }
            drop(state);
            async_std::task::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn request_init(&mut self) -> Option<Blockchain> {
        let mut tries = 0;

        loop {
            if tries == 5 {
                return None;
            }

            let (sender, receiver) = oneshot::channel();
            let command = Command::RequestInit { sender };

            let _ = self.send_if_ready(command).await;

            let res = receiver.await.expect("sender not to be dropped");

            match res {
                Ok(blockchain) => {
                    return blockchain;
                }
                Err(_) => {
                    async_std::task::sleep(Duration::from_secs(1)).await;
                }
            }
            tries += 1;
        }
    }
}
