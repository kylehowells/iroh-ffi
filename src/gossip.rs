use std::sync::Arc;

use bytes::Bytes;
use futures::StreamExt;
use iroh::EndpointId;
use iroh_gossip::api::Event;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::node::Iroh;
use crate::{CallbackError, IrohError};

/// Gossip message
#[derive(Debug, uniffi::Object)]
pub enum Message {
    /// We have a new, direct neighbor in the swarm membership layer for this topic
    NeighborUp(String),
    /// We dropped direct neighbor in the swarm membership layer for this topic
    NeighborDown(String),
    /// A gossip message was received for this topic
    Received {
        /// The content of the message
        content: Vec<u8>,
        /// The node that delivered the message. This is not the same as the original author.
        delivered_from: String,
    },
    /// We missed some messages
    Lagged,
    /// There was a gossip error
    Error(String),
}

#[derive(Debug, uniffi::Enum)]
pub enum MessageType {
    NeighborUp,
    NeighborDown,
    Received,
    Lagged,
    Error,
}

#[uniffi::export]
impl Message {
    pub fn r#type(&self) -> MessageType {
        match self {
            Self::NeighborUp(_) => MessageType::NeighborUp,
            Self::NeighborDown(_) => MessageType::NeighborDown,
            Self::Received { .. } => MessageType::Received,
            Self::Lagged => MessageType::Lagged,
            Self::Error(_) => MessageType::Error,
        }
    }

    pub fn as_neighbor_up(&self) -> String {
        if let Self::NeighborUp(s) = self {
            s.clone()
        } else {
            panic!("not a NeighborUp message");
        }
    }

    pub fn as_neighbor_down(&self) -> String {
        if let Self::NeighborDown(s) = self {
            s.clone()
        } else {
            panic!("not a NeighborDown message");
        }
    }

    pub fn as_received(&self) -> MessageContent {
        if let Self::Received {
            content,
            delivered_from,
        } = self
        {
            MessageContent {
                content: content.clone(),
                delivered_from: delivered_from.clone(),
            }
        } else {
            panic!("not a Received message");
        }
    }

    pub fn as_error(&self) -> String {
        if let Self::Error(s) = self {
            s.clone()
        } else {
            panic!("not a Error message");
        }
    }
}

/// The actual content of a gossip message.
#[derive(Debug, uniffi::Record)]
pub struct MessageContent {
    /// The content of the message
    pub content: Vec<u8>,
    /// The node that delivered the message. This is not the same as the original author.
    pub delivered_from: String,
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait GossipMessageCallback: Send + Sync + 'static {
    async fn on_message(&self, msg: Arc<Message>) -> Result<(), CallbackError>;
}

/// Iroh gossip client.
#[derive(uniffi::Object)]
pub struct Gossip {
    gossip: iroh_gossip::net::Gossip,
}

#[uniffi::export]
impl Iroh {
    /// Access to gossip specific functionality.
    pub fn gossip(&self) -> Gossip {
        let gossip = self.gossip.clone();
        Gossip { gossip }
    }
}

#[uniffi::export]
impl Gossip {
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn subscribe(
        &self,
        topic: Vec<u8>,
        bootstrap: Vec<String>,
        cb: Arc<dyn GossipMessageCallback>,
    ) -> Result<Sender, IrohError> {
        if topic.len() != 32 {
            return Err(anyhow::anyhow!("topic must be exactly 32 bytes").into());
        }
        let topic_bytes: [u8; 32] = topic.try_into().unwrap();

        let bootstrap = bootstrap
            .into_iter()
            .map(|b| b.parse())
            .collect::<Result<Vec<EndpointId>, _>>()
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        // Use subscribe instead of subscribe_and_join to avoid blocking
        // subscribe_and_join waits for at least one peer connection, which can block forever
        // if peers aren't immediately reachable
        let topic_handle = self
            .gossip
            .subscribe(topic_bytes.into(), bootstrap)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let (sender, mut receiver) = topic_handle.split();

        let cancel_token = CancellationToken::new();
        let cancel = cancel_token.clone();
        tokio::task::spawn(async move {
            tracing::debug!("gossip receiver task started");
            loop {
                tokio::select! {
                    biased;

                    _ = cancel.cancelled() => {
                        tracing::debug!("gossip receiver task cancelled");
                        break;
                    }
                    event = receiver.next() => {
                        match event {
                            Some(Ok(Event::NeighborUp(n))) => {
                                let message = Message::NeighborUp(n.to_string());
                                if let Err(err) = cb.on_message(Arc::new(message)).await {
                                    warn!("cb error, gossip: {:?}", err);
                                }
                            }
                            Some(Ok(Event::NeighborDown(n))) => {
                                let message = Message::NeighborDown(n.to_string());
                                if let Err(err) = cb.on_message(Arc::new(message)).await {
                                    warn!("cb error, gossip: {:?}", err);
                                }
                            }
                            Some(Ok(Event::Received(msg))) => {
                                let message = Message::Received {
                                    content: msg.content.to_vec(),
                                    delivered_from: msg.delivered_from.to_string(),
                                };
                                if let Err(err) = cb.on_message(Arc::new(message)).await {
                                    warn!("cb error, gossip: {:?}", err);
                                }
                            }
                            Some(Ok(Event::Lagged)) => {
                                let message = Message::Lagged;
                                if let Err(err) = cb.on_message(Arc::new(message)).await {
                                    warn!("cb error, gossip: {:?}", err);
                                }
                            }
                            Some(Err(err)) => {
                                let message = Message::Error(err.to_string());
                                if let Err(err) = cb.on_message(Arc::new(message)).await {
                                    warn!("cb error, gossip: {:?}", err);
                                }
                            }
                            None => {
                                tracing::debug!("gossip receiver stream ended");
                                break;
                            }
                        }
                    }
                }
            }
        });

        let sender = Sender {
            sender: Mutex::new(sender),
            cancel: cancel_token,
        };

        Ok(sender)
    }
}

/// Gossip sender
#[derive(uniffi::Object)]
pub struct Sender {
    sender: Mutex<iroh_gossip::api::GossipSender>,
    cancel: CancellationToken,
}

#[uniffi::export]
impl Sender {
    /// Broadcast a message to all nodes in the swarm
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn broadcast(&self, msg: Vec<u8>) -> Result<(), IrohError> {
        self.sender
            .lock()
            .await
            .broadcast(Bytes::from(msg))
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(())
    }

    /// Broadcast a message to all direct neighbors.
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn broadcast_neighbors(&self, msg: Vec<u8>) -> Result<(), IrohError> {
        self.sender
            .lock()
            .await
            .broadcast_neighbors(Bytes::from(msg))
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(())
    }

    /// Closes the subscription, it is an error to use it afterwards
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn cancel(&self) -> Result<(), IrohError> {
        if self.cancel.is_cancelled() {
            return Err(IrohError::from(anyhow::anyhow!("already closed")));
        }
        self.cancel.cancel();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::*;

    #[tokio::test]
    async fn test_gossip_basic() {
        // Enable tracing for debugging
        let _ = tracing_subscriber::fmt::try_init();

        let n0 = Iroh::memory().await.unwrap();
        let n1 = Iroh::memory().await.unwrap();

        struct Cb {
            name: &'static str,
            channel: mpsc::Sender<Arc<Message>>,
        }
        #[async_trait::async_trait]
        impl GossipMessageCallback for Cb {
            async fn on_message(&self, message: Arc<Message>) -> Result<(), CallbackError> {
                println!("{} << {:?}", self.name, message);
                self.channel.send(message).await.unwrap();
                Ok(())
            }
        }

        let topic = [1u8; 32].to_vec();

        // Wait for nodes to be online (connected to relay and have direct addresses)
        n0.net().wait_online().await.unwrap();
        n1.net().wait_online().await.unwrap();

        // Get addresses and IDs for both nodes
        let n0_id = n0.net().node_id();
        let n0_addr = Arc::new(n0.net().node_addr());
        let n1_id = n1.net().node_id();
        let n1_addr = Arc::new(n1.net().node_addr());
        println!("n0 addr: {:?}", n0_addr);
        println!("n1 addr: {:?}", n1_addr);

        // Add addresses to both static providers BEFORE subscribing
        n0.net().add_node_addr(n1_addr).unwrap();
        n1.net().add_node_addr(n0_addr).unwrap();

        // n0 subscribes first with empty bootstrap
        let (sender0, mut receiver0) = mpsc::channel(8);
        let cb0 = Cb { name: "n0", channel: sender0 };
        println!("subscribing n0 to topic (no bootstrap)");
        let sink0 = n0
            .gossip()
            .subscribe(topic.clone(), vec![], Arc::new(cb0))
            .await
            .unwrap();
        println!("n0 subscribed");

        // n1 subscribes with n0 as bootstrap - this should initiate connection from n1 to n0
        let (sender1, mut receiver1) = mpsc::channel(8);
        let cb1 = Cb { name: "n1", channel: sender1 };
        println!("subscribing n1 to topic with n0 as bootstrap");
        let _sink1 = n1
            .gossip()
            .subscribe(topic.clone(), vec![n0_id.clone()], Arc::new(cb1))
            .await
            .unwrap();
        println!("n1 subscribed");

        // Wait for n0 to see n1 as a neighbor
        // Note: In gossip, the connecting node (n1) may not get NeighborUp until it receives
        // a message, so we only wait for n0's NeighborUp
        let wait_neighbor0 = async {
            loop {
                let Some(event) = receiver0.recv().await else {
                    panic!("receiver0 stream closed");
                };
                println!("n0 event: {:?}", event);
                if matches!(&*event, Message::NeighborUp(_)) {
                    break;
                }
            }
        };

        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            wait_neighbor0
        )
            .await
            .expect("timeout waiting for n0 to see neighbor");

        // Give time for the gossip protocol to fully establish
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Send message on n0
        println!("sending message");
        let msg_content = b"hello";
        sink0.broadcast(msg_content.to_vec()).await.unwrap();

        // Receive on n1
        let recv_fut = async {
            loop {
                let Some(event) = receiver1.recv().await else {
                    panic!("receiver stream closed before receiving gossip message");
                };
                println!("event: {:?}", event);
                if let Message::Received {
                    ref content,
                    ref delivered_from,
                } = &*event
                {
                    assert_eq!(content, msg_content);
                    assert_eq!(delivered_from, &n0_id.to_string());

                    break;
                }
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(15), recv_fut)
            .await
            .expect("timeout reached and no gossip message received");
    }
}
