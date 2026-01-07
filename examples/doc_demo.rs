//! Rust document demo using iroh-ffi
//!
//! Usage:
//!   cargo run --example doc_demo -- create
//!   cargo run --example doc_demo -- join [TICKET]
//!
//! Examples:
//!   # Create a document and wait for peers
//!   cargo run --example doc_demo -- create
//!
//!   # Join an existing document
//!   cargo run --example doc_demo -- join <TICKET>

use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

use iroh_ffi::{
    AddrInfoOptions, CallbackError, DocTicket, Iroh, LiveEvent, LiveEventType, NodeOptions,
    Query, ShareMode, SubscribeCallback,
};

struct DocCallback {
    name: String,
}

#[async_trait::async_trait]
impl SubscribeCallback for DocCallback {
    async fn event(&self, event: Arc<LiveEvent>) -> Result<(), CallbackError> {
        match event.r#type() {
            LiveEventType::InsertLocal => {
                let entry = event.as_insert_local();
                let key_bytes = entry.key();
                let key = String::from_utf8_lossy(&key_bytes);
                println!(
                    "\n[{}] Local insert: key='{}', content_len={}",
                    self.name,
                    key,
                    entry.content_len()
                );
            }
            LiveEventType::InsertRemote => {
                let insert = event.as_insert_remote();
                let key_bytes = insert.entry.key();
                let key = String::from_utf8_lossy(&key_bytes);
                println!(
                    "\n[{}] Remote insert from {}: key='{}', content_len={}",
                    self.name,
                    insert.from,
                    key,
                    insert.entry.content_len()
                );
            }
            LiveEventType::ContentReady => {
                let hash = event.as_content_ready();
                println!("\n[{}] Content ready: {}", self.name, hash);
            }
            LiveEventType::SyncFinished => {
                let sync = event.as_sync_finished();
                println!("\n[{}] Sync finished with peer: {}", self.name, sync.peer);
            }
            LiveEventType::NeighborUp => {
                let peer = event.as_neighbor_up();
                println!("\n[{}] Neighbor up: {}", self.name, peer);
            }
            LiveEventType::NeighborDown => {
                let peer = event.as_neighbor_down();
                println!("\n[{}] Neighbor down: {}", self.name, peer);
            }
            LiveEventType::PendingContentReady => {
                println!("\n[{}] Pending content ready", self.name);
            }
        }
        print!("> ");
        io::stdout().flush().ok();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    // Create node with docs enabled
    let temp_dir = std::env::temp_dir().join(format!("iroh-doc-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)?;

    println!("=== Iroh Doc Demo (Rust) ===\n");
    println!("Creating node at {:?}...", temp_dir);

    let options = NodeOptions {
        enable_docs: true,
        ..Default::default()
    };
    let node = Iroh::persistent_with_options(temp_dir.to_string_lossy().to_string(), options).await?;

    // Wait for node to be online
    println!("Waiting for node to come online...");
    node.net().wait_online().await?;

    let my_id = node.net().node_id();
    let my_addr = node.net().node_addr();
    println!("Node ID: {}", my_id);
    println!("Relay URL: {:?}\n", my_addr.relay_url());

    // Get or create an author
    let authors = node.authors().list().await?;
    let author = if let Some(author) = authors.first() {
        println!("Using existing author: {}", author);
        author.clone()
    } else {
        let new_author = node.authors().create().await?;
        println!("Created new author: {}", new_author);
        new_author
    };

    match command.as_str() {
        "create" => {
            create_and_host_doc(&node, &author).await?;
        }
        "join" => {
            if args.len() < 3 {
                eprintln!("Usage: cargo run --example doc_demo -- join [TICKET]");
                return Ok(());
            }
            let ticket_str = &args[2];
            join_doc(&node, &author, ticket_str).await?;
        }
        _ => {
            print_usage();
        }
    }

    println!("\nShutting down...");
    node.node().shutdown().await?;

    Ok(())
}

fn print_usage() {
    println!("Usage:");
    println!("  cargo run --example doc_demo -- create");
    println!("  cargo run --example doc_demo -- join [TICKET]");
}

async fn create_and_host_doc(
    node: &Iroh,
    author: &iroh_ffi::AuthorId,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating new document...");
    let doc = node.docs().create().await?;
    println!("Document ID: {}", doc.id());

    // Subscribe to events
    let callback = DocCallback {
        name: "doc".to_string(),
    };

    doc.subscribe(Arc::new(callback)).await?;
    println!("Subscribed to document events");

    // Set some initial entries
    println!("\nSetting initial entries...");
    let _ = doc.set_bytes(author, b"greeting".to_vec(), b"Hello from Rust!".to_vec()).await?;
    let _ = doc.set_bytes(author, b"count".to_vec(), b"0".to_vec()).await?;
    println!("Initial entries set");

    // Create and share ticket
    let ticket = doc.share(ShareMode::Write, AddrInfoOptions::RelayAndAddresses).await?;

    println!("\n=== DOC TICKET ===");
    println!("{}", ticket);
    println!("==================\n");
    println!("To join this document, run:");
    println!("  cargo run --example doc_demo -- join {}", ticket);

    println!("\n=== Interactive Mode ===");
    println!("Commands:");
    println!("  set <key> <value>  - Set a key-value pair");
    println!("  get <key>          - Get a value by key");
    println!("  list               - List all entries");
    println!("  /quit              - Exit\n");

    // Interactive loop
    print!("> ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    for line in stdin.lines() {
        let line = line?;
        let parts: Vec<&str> = line.trim().split_whitespace().collect();

        if parts.is_empty() {
            print!("> ");
            io::stdout().flush()?;
            continue;
        }

        match parts[0] {
            "/quit" | "quit" | "exit" => break,
            "set" if parts.len() >= 3 => {
                let key = parts[1].as_bytes().to_vec();
                let value = parts[2..].join(" ").into_bytes();
                let hash = doc.set_bytes(author, key, value).await?;
                println!("Set '{}' = '{}' (hash: {})", parts[1], parts[2..].join(" "), hash);
            }
            "get" if parts.len() >= 2 => {
                let key = parts[1].as_bytes().to_vec();
                let query = Query::key_exact(key, None);
                if let Some(entry) = doc.get_one(Arc::new(query)).await? {
                    let content = node.blobs().read_to_bytes(entry.content_hash()).await?;
                    let value = String::from_utf8_lossy(&content);
                    println!("'{}' = '{}'", parts[1], value);
                } else {
                    println!("Key '{}' not found", parts[1]);
                }
            }
            "list" => {
                let query = Query::all(None);
                let entries = doc.get_many(Arc::new(query)).await?;
                println!("Entries ({}):", entries.len());
                for entry in entries {
                    let key_bytes = entry.key();
                    let key = String::from_utf8_lossy(&key_bytes);
                    let content = node.blobs().read_to_bytes(entry.content_hash()).await?;
                    let value = String::from_utf8_lossy(&content);
                    println!("  '{}' = '{}'", key, value);
                }
            }
            _ => {
                println!("Unknown command. Try: set, get, list, /quit");
            }
        }

        print!("> ");
        io::stdout().flush()?;
    }

    Ok(())
}

async fn join_doc(
    node: &Iroh,
    author: &iroh_ffi::AuthorId,
    ticket_str: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Parsing ticket...");
    let ticket = DocTicket::new(ticket_str.to_string())?;

    println!("\nJoining document...");

    let callback = DocCallback {
        name: "doc".to_string(),
    };

    let doc = node.docs().join_and_subscribe(&ticket, Arc::new(callback)).await?;
    println!("Joined document: {}", doc.id());

    // Give it a moment to sync
    tokio::time::sleep(Duration::from_secs(2)).await;

    // List existing entries
    println!("\nExisting entries:");
    let query = Query::all(None);
    let entries = doc.get_many(Arc::new(query)).await?;
    for entry in &entries {
        let key_bytes = entry.key();
        let key = String::from_utf8_lossy(&key_bytes);
        let content = node.blobs().read_to_bytes(entry.content_hash()).await?;
        let value = String::from_utf8_lossy(&content);
        println!("  '{}' = '{}'", key, value);
    }

    println!("\n=== Interactive Mode ===");
    println!("Commands:");
    println!("  set <key> <value>  - Set a key-value pair");
    println!("  get <key>          - Get a value by key");
    println!("  list               - List all entries");
    println!("  /quit              - Exit\n");

    // Interactive loop
    print!("> ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    for line in stdin.lines() {
        let line = line?;
        let parts: Vec<&str> = line.trim().split_whitespace().collect();

        if parts.is_empty() {
            print!("> ");
            io::stdout().flush()?;
            continue;
        }

        match parts[0] {
            "/quit" | "quit" | "exit" => break,
            "set" if parts.len() >= 3 => {
                let key = parts[1].as_bytes().to_vec();
                let value = parts[2..].join(" ").into_bytes();
                let hash = doc.set_bytes(author, key, value).await?;
                println!("Set '{}' = '{}' (hash: {})", parts[1], parts[2..].join(" "), hash);
            }
            "get" if parts.len() >= 2 => {
                let key = parts[1].as_bytes().to_vec();
                let query = Query::key_exact(key, None);
                if let Some(entry) = doc.get_one(Arc::new(query)).await? {
                    let content = node.blobs().read_to_bytes(entry.content_hash()).await?;
                    let value = String::from_utf8_lossy(&content);
                    println!("'{}' = '{}'", parts[1], value);
                } else {
                    println!("Key '{}' not found", parts[1]);
                }
            }
            "list" => {
                let query = Query::all(None);
                let entries = doc.get_many(Arc::new(query)).await?;
                println!("Entries ({}):", entries.len());
                for entry in entries {
                    let key_bytes = entry.key();
                    let key = String::from_utf8_lossy(&key_bytes);
                    let content = node.blobs().read_to_bytes(entry.content_hash()).await?;
                    let value = String::from_utf8_lossy(&content);
                    println!("  '{}' = '{}'", key, value);
                }
            }
            _ => {
                println!("Unknown command. Try: set, get, list, /quit");
            }
        }

        print!("> ");
        io::stdout().flush()?;
    }

    Ok(())
}
