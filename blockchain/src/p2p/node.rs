use futures::channel::{mpsc, oneshot};
use futures::prelude::*;
use futures::stream::StreamExt;
use libp2p::request_response::OutboundRequestId;
use libp2p::{
    gossipsub, mdns, noise,
    request_response::{self, ProtocolSupport},
    swarm::NetworkBehaviour,
    swarm::SwarmEvent,
    tcp, yamux, Multiaddr, StreamProtocol,
};
use std::collections::HashMap;

use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;
use tokio::io;

use tokio::sync::Mutex;

use crate::models::blockchain::Blockchain;

use super::{Command, InitRequest, InitResponse, NodeState};

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    // for init
    request_response: request_response::cbor::Behaviour<InitRequest, InitResponse>,
    // for chain updates
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

pub struct Node {
    state: Arc<Mutex<NodeState>>,
    swarm: libp2p::Swarm<MyBehaviour>,
    receiver: mpsc::Receiver<Command>,
    pending_init_requests:
        HashMap<OutboundRequestId, oneshot::Sender<Result<Option<Blockchain>, ()>>>,
}

impl Node {
    pub async fn new(
        receiver: mpsc::Receiver<Command>,
        node_state: Arc<Mutex<NodeState>>,
    ) -> Result<Node, Box<dyn Error>> {
        let swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                // To content-address message, we can take the hash of message and use it as an ID.
                let message_id_fn = |message: &gossipsub::Message| {
                    let mut s = DefaultHasher::new();
                    message.data.hash(&mut s);
                    gossipsub::MessageId::from(s.finish().to_string())
                };

                // Set a custom gossipsub configuration
                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
                    .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
                    .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
                    .build()
                    .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?; // Temporary hack because `build` does not return a proper `std::error::Error`.

                // build a gossipsub network behaviour
                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )?;

                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?;

                let request_response = request_response::cbor::Behaviour::new(
                    [(StreamProtocol::new("/init"), ProtocolSupport::Full)],
                    request_response::Config::default(),
                );

                Ok(MyBehaviour {
                    gossipsub,
                    mdns,
                    request_response,
                })
            })?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        let pending_requests_init = HashMap::new();

        Ok(Node {
            swarm,
            receiver,
            state: node_state,
            pending_init_requests: pending_requests_init,
        })
    }

    pub async fn start(mut self, addr: Option<Multiaddr>) {
        let init_topic = gossipsub::IdentTopic::new("init");
        let chain_update_topic = gossipsub::IdentTopic::new("chain-update");

        let addr = addr.unwrap_or("/ip4/0.0.0.0/tcp/0".parse().unwrap());

        let _ = self.swarm.behaviour_mut().gossipsub.subscribe(&init_topic);
        let _ = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&chain_update_topic);

        println!("subscribed to init and chain-update");

        let _ = self.swarm.listen_on(addr.clone());

        println!("Started listening on {addr}");

        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => self.handle_swarm_event(event).await,
                command = self.receiver.next() => match command {
                    Some(c) => self.handle_command(c).await,
                    None =>  { return; },
                },
            }
        }
    }

    async fn handle_swarm_event(&mut self, event: SwarmEvent<MyBehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(MyBehaviourEvent::RequestResponse(
                request_response::Event::Message { message, .. },
            )) => {
                match message {
                    request_response::Message::Request { channel, .. } => {
                        let state = self.state.lock().await;
                        let init_response = if state.blockchain.chain.is_empty() {
                            InitResponse(None)
                        } else {
                            InitResponse(Some(state.blockchain.clone()))
                        };

                        let _ = self
                            .swarm
                            .behaviour_mut()
                            .request_response
                            .send_response(channel, init_response);
                        println!("sent response");
                    }
                    request_response::Message::Response {
                        request_id,
                        response,
                        ..
                    } => {
                        if let Some(pending) = self.pending_init_requests.remove(&request_id) {
                            match pending.send(Ok(response.0)) {
                                Err(e) => {
                                    eprintln!("{:?}", e)
                                }
                                _ => {}
                            }
                        };
                    }
                };
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source: peer_id,
                message_id: id,
                message,
            })) => {
                println!(
                    "Got message: '{}' with id: {id} from peer: {peer_id}",
                    String::from_utf8_lossy(&message.data)
                );
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, _multiaddr) in list {
                    // println!("mDNS discovered a new peer: {peer_id}");
                    let behaviour = self.swarm.behaviour_mut();
                    behaviour.gossipsub.add_explicit_peer(&peer_id);
                }
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                for (peer_id, _multiaddr) in list {
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                }
            }
            SwarmEvent::NewListenAddr { .. } => {
                let mut state = self.state.lock().await;
                state.ready = true;
                // println!("Local node is listening on {address}");
            }
            _ => {}
        }
    }

    async fn handle_command(&mut self, command: Command) {
        match command {
            Command::RequestInit { sender } => {
                let behaviour = self.swarm.behaviour_mut();
                let nodes = behaviour.mdns.discovered_nodes();

                if nodes.len() != 0 {
                    let receivers: Vec<_> = nodes
                        .take(3) // request from a max of 3 nodes
                        .into_iter()
                        .map(|n| {
                            let (our_sender, our_receiver) = oneshot::channel();
                            let init_request = InitRequest("init".to_owned());
                            let request_id =
                                behaviour.request_response.send_request(n, init_request);
                            self.pending_init_requests.insert(request_id, our_sender);

                            our_receiver
                        })
                        .collect();

                    let requests = receivers.into_iter().map(|r| async { r.await }.boxed());

                    // spawn this in a new thread since we dont want to block handling of swarm events
                    tokio::task::spawn(async {
                        // Await the requests, ignore the remaining once a single one succeeds.
                        match futures::future::select_ok(requests)
                            .await
                            .map_err(|_| "None of the providers returned file.")
                        {
                            Ok(requests) => {
                                let _ = sender.send(requests.0);
                            }
                            Err(_) => {
                                let _ = sender.send(Err(()));
                            }
                        };
                    });
                } else {
                    println!("no nodes");
                    let _ = sender.send(Err(()));
                }

                // if flag == true {
                //                     // } else {
                // }
            }
        }
    }

    async fn _compare_chains_and_save(&mut self, chain: &Blockchain) {
        let mut state = self.state.lock().await;
        let better_chain = Blockchain::compare(&state.blockchain, chain);

        state.blockchain = better_chain.clone();
    }
}
