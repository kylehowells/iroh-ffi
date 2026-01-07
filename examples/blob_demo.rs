//! Rust blob demo using iroh-ffi
//!
//! Usage:
//!   cargo run --example blob_demo -- send [FILE]
//!   cargo run --example blob_demo -- receive [TICKET] [DEST_FILE]
//!   cargo run --example blob_demo -- send-bytes [TEXT]
//!
//! Examples:
//!   # Send a file
//!   cargo run --example blob_demo -- send ./myfile.txt
//!
//!   # Receive a file using a ticket
//!   cargo run --example blob_demo -- receive <TICKET> ./downloaded.txt
//!
//!   # Send some text as bytes
//!   cargo run --example blob_demo -- send-bytes "Hello, iroh!"

use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use iroh_ffi::{
    AddCallback, AddProgress, AddProgressType, BlobFormat, BlobTicket, DownloadCallback,
    DownloadProgress, DownloadProgressType, Iroh, SetTagOption, WrapOption,
};

struct PrintAddCallback {
    name: String,
}

#[async_trait::async_trait]
impl AddCallback for PrintAddCallback {
    async fn progress(
        &self,
        progress: Arc<AddProgress>,
    ) -> Result<(), iroh_ffi::CallbackError> {
        match progress.r#type() {
            AddProgressType::Found => {
                let found = progress.as_found();
                println!("[{}] Found: {} ({} bytes)", self.name, found.name, found.size);
            }
            AddProgressType::Progress => {
                let p = progress.as_progress();
                print!("\r[{}] Progress: {} bytes", self.name, p.offset);
                io::stdout().flush().ok();
            }
            AddProgressType::Done => {
                let done = progress.as_done();
                println!("\n[{}] Done: {}", self.name, done.hash);
            }
            AddProgressType::AllDone => {
                let all_done = progress.as_all_done();
                println!(
                    "[{}] All done! Hash: {}, Format: {:?}",
                    self.name, all_done.hash, all_done.format
                );
            }
            AddProgressType::Abort => {
                let abort = progress.as_abort();
                eprintln!("[{}] Aborted: {}", self.name, abort.error);
            }
        }
        Ok(())
    }
}

struct PrintDownloadCallback {
    name: String,
}

#[async_trait::async_trait]
impl DownloadCallback for PrintDownloadCallback {
    async fn progress(
        &self,
        progress: Arc<DownloadProgress>,
    ) -> Result<(), iroh_ffi::CallbackError> {
        match progress.r#type() {
            DownloadProgressType::Connected => {
                println!("[{}] Connected to peer", self.name);
            }
            DownloadProgressType::Found => {
                let found = progress.as_found();
                println!(
                    "[{}] Found blob: {} ({} bytes)",
                    self.name, found.hash, found.size
                );
            }
            DownloadProgressType::Progress => {
                let p = progress.as_progress();
                print!("\r[{}] Download progress: {} bytes", self.name, p.offset);
                io::stdout().flush().ok();
            }
            DownloadProgressType::Done => {
                println!("\n[{}] Blob download complete", self.name);
            }
            DownloadProgressType::AllDone => {
                let all_done = progress.as_all_done();
                println!(
                    "[{}] All done! {} bytes written, {} bytes read, {:?} elapsed",
                    self.name, all_done.bytes_written, all_done.bytes_read, all_done.elapsed
                );
            }
            DownloadProgressType::Abort => {
                let abort = progress.as_abort();
                eprintln!("[{}] Download aborted: {}", self.name, abort.error);
            }
            _ => {}
        }
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

    // Create node with persistent storage in temp directory
    let temp_dir = std::env::temp_dir().join(format!("iroh-blob-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir)?;

    println!("=== Iroh Blob Demo (Rust) ===\n");
    println!("Creating node at {:?}...", temp_dir);
    let node = Iroh::persistent(temp_dir.to_string_lossy().to_string()).await?;

    // Wait for node to be online
    println!("Waiting for node to come online...");
    node.net().wait_online().await?;

    let my_id = node.net().node_id();
    let my_addr = node.net().node_addr();
    println!("Node ID: {}", my_id);
    println!("Relay URL: {:?}\n", my_addr.relay_url());

    match command.as_str() {
        "send" => {
            if args.len() < 3 {
                eprintln!("Usage: cargo run --example blob_demo -- send [FILE]");
                return Ok(());
            }
            let file_path = &args[2];
            send_file(&node, file_path).await?;

            println!("\nWaiting for peer to download... Press Ctrl+C to exit.");
            tokio::signal::ctrl_c().await?;
        }
        "send-bytes" => {
            if args.len() < 3 {
                eprintln!("Usage: cargo run --example blob_demo -- send-bytes [TEXT]");
                return Ok(());
            }
            let text = args[2..].join(" ");
            send_bytes(&node, text.as_bytes()).await?;

            println!("\nWaiting for peer to download... Press Ctrl+C to exit.");
            tokio::signal::ctrl_c().await?;
        }
        "receive" => {
            if args.len() < 4 {
                eprintln!("Usage: cargo run --example blob_demo -- receive [TICKET] [DEST_FILE]");
                return Ok(());
            }
            let ticket_str = &args[2];
            let dest_path = &args[3];
            receive_blob(&node, ticket_str, dest_path).await?;
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
    println!("  cargo run --example blob_demo -- send [FILE]");
    println!("  cargo run --example blob_demo -- receive [TICKET] [DEST_FILE]");
    println!("  cargo run --example blob_demo -- send-bytes [TEXT]");
}

async fn send_file(node: &Iroh, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Adding file: {}", file_path);

    let abs_path = std::path::absolute(PathBuf::from(file_path))?;
    let abs_path_str = abs_path.to_string_lossy().to_string();

    let callback = PrintAddCallback {
        name: "add".to_string(),
    };

    node.blobs()
        .add_from_path(
            abs_path_str,
            false, // copy, not in-place
            Arc::new(SetTagOption::auto()),
            Arc::new(WrapOption::no_wrap()),
            Arc::new(callback),
        )
        .await?;

    // After adding, list blobs to get the hash
    let blobs = node.blobs().list().await?;
    if let Some(hash) = blobs.first() {
        // Create a ticket for sharing
        let ticket = node
            .blobs()
            .share(
                hash.clone(),
                BlobFormat::Raw,
                iroh_ffi::AddrInfoOptions::Id,
            )
            .await?;

        println!("\n=== BLOB TICKET ===");
        println!("{}", ticket.to_string());
        println!("===================\n");
        println!("To download this file, run:");
        println!(
            "  cargo run --example blob_demo -- receive {} [DEST_FILE]",
            ticket.to_string()
        );
    }

    Ok(())
}

async fn send_bytes(node: &Iroh, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    println!("Adding {} bytes of data...", data.len());

    let outcome = node.blobs().add_bytes(data.to_vec()).await?;

    println!("Added blob with hash: {}", outcome.hash);
    println!("Format: {:?}", outcome.format);
    println!("Size: {} bytes", outcome.size);

    // Create a ticket for sharing
    let ticket = node
        .blobs()
        .share(
            outcome.hash.clone(),
            BlobFormat::Raw,
            iroh_ffi::AddrInfoOptions::Id,
        )
        .await?;

    println!("\n=== BLOB TICKET ===");
    println!("{}", ticket.to_string());
    println!("===================\n");
    println!("To download this blob, run:");
    println!(
        "  cargo run --example blob_demo -- receive {} [DEST_FILE]",
        ticket.to_string()
    );

    // Also show the content for text data
    if let Ok(text) = String::from_utf8(data.to_vec()) {
        println!("\nBlob content: \"{}\"", text);
    }

    Ok(())
}

async fn receive_blob(
    node: &Iroh,
    ticket_str: &str,
    dest_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Parsing ticket...");
    let ticket = BlobTicket::new(ticket_str.to_string())?;

    let hash = ticket.hash();
    let addr = ticket.node_addr();

    println!("Blob hash: {}", hash);
    println!("Provider relay URL: {:?}", addr.relay_url());

    // Add the node address to discovery for direct connection
    node.net().add_node_addr(Arc::new((*addr).clone()))?;

    println!("\nDownloading blob...");

    let download_opts = iroh_ffi::BlobDownloadOptions::new(
        BlobFormat::Raw,
        vec![Arc::new((*addr).clone())],
        Arc::new(SetTagOption::auto()),
    )?;

    let callback = PrintDownloadCallback {
        name: "download".to_string(),
    };

    node.blobs()
        .download(hash.clone(), Arc::new(download_opts), Arc::new(callback))
        .await?;

    println!("\nExporting to file: {}", dest_path);
    node.blobs()
        .write_to_path(hash.clone(), dest_path.to_string())
        .await?;

    println!("File saved to: {}", dest_path);

    // If it's a small blob, show the content
    let size = node.blobs().size(&hash).await?;
    if size < 1024 {
        let data = node.blobs().read_to_bytes(hash).await?;
        if let Ok(text) = String::from_utf8(data) {
            println!("\nBlob content: \"{}\"", text);
        }
    }

    Ok(())
}
