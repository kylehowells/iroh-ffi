//! Rust gossip chat demo using iroh-ffi
//!
//! Usage:
//!   cargo run --example gossip_chat -- [TOPIC_HEX] [PEER_NODE_ID] [PEER_RELAY_URL]
//!
//! If no arguments provided, creates a new topic and prints node info.
//! If TOPIC_HEX provided, joins that topic.
//! If PEER_NODE_ID and PEER_RELAY_URL provided, connects to that peer.

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use iroh_ffi::{
    CallbackError, GossipMessageCallback, Iroh, Message, MessageType, NodeAddr, PublicKey,
};

struct ChatCallback {
    name: String,
}

#[async_trait::async_trait]
impl GossipMessageCallback for ChatCallback {
    async fn on_message(&self, message: Arc<Message>) -> Result<(), CallbackError> {
        match message.r#type() {
            MessageType::NeighborUp => {
                let peer = message.as_neighbor_up();
                println!("\n[{}] Peer connected: {}...", self.name, &peer[..16]);
            }
            MessageType::NeighborDown => {
                let peer = message.as_neighbor_down();
                println!("\n[{}] Peer disconnected: {}...", self.name, &peer[..16]);
            }
            MessageType::Received => {
                let msg = message.as_received();
                let content = String::from_utf8_lossy(&msg.content);
                println!("\n[{}] {}...: {}", self.name, &msg.delivered_from[..8], content);
            }
            MessageType::Lagged => {
                println!("\n[{}] Warning: missed some messages", self.name);
            }
            MessageType::Error => {
                let err = message.as_error();
                eprintln!("\n[{}] Error: {}", self.name, err);
            }
        }
        print!("> ");
        io::stdout().flush().ok();
        Ok(())
    }
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Hex string must have even length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    println!("=== Iroh Gossip Chat Demo (Rust) ===\n");

    // Create node with persistent storage in temp directory
    let temp_dir = std::env::temp_dir().join(format!("iroh-chat-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)?;

    println!("Creating node at {:?}...", temp_dir);
    let node = Iroh::persistent(temp_dir.to_string_lossy().to_string()).await?;

    // Wait for node to be online
    println!("Waiting for node to come online...");
    node.net().wait_online().await?;

    let my_id = node.net().node_id();
    let my_addr = node.net().node_addr();
    println!("Node ID: {}", my_id);
    println!("Relay URL: {:?}", my_addr.relay_url());
    println!("Direct addresses: {:?}\n", my_addr.direct_addresses());

    // Parse topic from args or generate random one
    let topic: Vec<u8> = if args.len() > 1 {
        let topic_hex = &args[1];
        hex_to_bytes(topic_hex)?
    } else {
        // Generate random topic
        let mut topic = vec![0u8; 32];
        for (i, byte) in topic.iter_mut().enumerate() {
            *byte = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos() as u8)
                .wrapping_add(i as u8);
        }
        topic
    };

    if topic.len() != 32 {
        eprintln!("Topic must be exactly 32 bytes (64 hex chars)");
        return Ok(());
    }

    println!("Topic: {}", bytes_to_hex(&topic));

    // If peer info provided, add it to discovery
    // Supports: TOPIC PEER_ID [RELAY_URL]
    let bootstrap: Vec<String> = if args.len() > 2 {
        let peer_node_id = &args[2];
        println!("Adding peer: {}...", &peer_node_id[..16.min(peer_node_id.len())]);

        // If relay URL is also provided, add to StaticProvider for faster discovery
        if args.len() > 3 {
            let peer_relay_url = &args[3];
            let peer_pubkey = PublicKey::from_string(peer_node_id.clone())?;
            let peer_addr = NodeAddr::new(&peer_pubkey, Some(peer_relay_url.clone()), vec![]);
            node.net().add_node_addr(Arc::new(peer_addr))?;
            println!("Added peer with relay URL");
        } else {
            println!("Using discovery to find peer...");
        }

        vec![peer_node_id.clone()]
    } else {
        vec![]
    };

    // Subscribe to gossip topic
    println!("\nJoining gossip topic...");
    let callback = ChatCallback {
        name: "chat".to_string(),
    };
    let sender = node
        .gossip()
        .subscribe(topic.clone(), bootstrap, Arc::new(callback))
        .await?;

    println!("\n=== Chat started! Type messages and press Enter ===");
    println!("Share this topic with others: {}", bytes_to_hex(&topic));
    println!("Share your node ID: {}", my_id);
    if let Some(relay) = my_addr.relay_url() {
        println!("Share your relay URL: {}", relay);
    }
    println!("\nCommands: /id, /quit\n");

    // Read from stdin and broadcast
    print!("> ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            print!("> ");
            io::stdout().flush()?;
            continue;
        }

        if line.trim() == "/quit" {
            break;
        }

        if line.trim() == "/id" {
            println!("Your ID: {}", my_id);
            if let Some(relay) = my_addr.relay_url() {
                println!("Relay URL: {}", relay);
            }
            print!("> ");
            io::stdout().flush()?;
            continue;
        }

        // Broadcast message
        sender.broadcast(line.as_bytes().to_vec()).await?;
        print!("> ");
        io::stdout().flush()?;
    }

    println!("\nShutting down...");
    let _ = sender.cancel().await;
    node.node().shutdown().await?;

    Ok(())
}
