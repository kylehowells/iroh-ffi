use std::{collections::HashMap, fmt::Debug, path::PathBuf, sync::Arc, time::Duration};

use iroh_blobs::{
    BlobsProtocol,
    provider::events::EventSender,
};
use iroh_docs::protocol::Docs;
use iroh_gossip::net::Gossip;
use iroh::address_lookup::{DnsAddressLookup, MemoryLookup, PkarrPublisher};

use crate::{
    BlobProvideEventCallback, CallbackError, Connection, Endpoint, IrohError,
};

/// Stats counter
#[derive(Debug, uniffi::Record)]
pub struct CounterStats {
    /// The counter value
    pub value: u32,
    /// The counter description
    pub description: String,
}

// Note: DirectAddrInfo, ConnectionType, RemoteInfo, ConnType, LatencyAndControlMsg,
// and ConnectionTypeMixed have been removed in iroh 0.96.
// DirectAddrInfo and ConnectionType no longer exist in iroh::endpoint.
// Use Connection::paths() with TransportAddr for connection path inspection instead.

/// Options passed to [`IrohNode.new`]. Controls the behaviour of an iroh node.
#[derive(derive_more::Debug, uniffi::Record)]
pub struct NodeOptions {
    /// How frequently the blob store should clean up unreferenced blobs, in milliseconds.
    /// Set to 0 to disable gc
    #[uniffi(default = None)]
    pub gc_interval_millis: Option<u64>,
    /// Provide a callback to hook into events when the blobs component adds and provides blobs.
    #[debug("BlobProvideEventCallback")]
    #[uniffi(default = None)]
    pub blob_events: Option<Arc<dyn BlobProvideEventCallback>>,
    /// Should docs be enabled? Defaults to `false`.
    #[uniffi(default = false)]
    pub enable_docs: bool,
    /// Overwrites the default IPv4 address to bind to
    #[uniffi(default = None)]
    pub ipv4_addr: Option<String>,
    /// Overwrites the default IPv6 address to bind to
    #[uniffi(default = None)]
    pub ipv6_addr: Option<String>,
    /// Configure the node discovery. Defaults to the default set of config
    #[uniffi(default = None)]
    pub node_discovery: Option<NodeDiscoveryConfig>,
    /// Provide a specific secret key, identifying this node. Must be 32 bytes long.
    #[uniffi(default = None)]
    pub secret_key: Option<Vec<u8>>,

    #[uniffi(default = None)]
    pub protocols: Option<HashMap<Vec<u8>, Arc<dyn ProtocolCreator>>>,
}

#[uniffi::export(with_foreign)]
pub trait ProtocolCreator: std::fmt::Debug + Send + Sync + 'static {
    fn create(&self, endpoint: Arc<Endpoint>) -> Arc<dyn ProtocolHandler>;
}

#[uniffi::export(with_foreign)]
#[async_trait::async_trait]
pub trait ProtocolHandler: Send + Sync + 'static {
    async fn accept(&self, conn: Arc<Connection>) -> Result<(), CallbackError>;
    async fn shutdown(&self);
}

#[derive(derive_more::Debug, Clone)]
struct ProtocolWrapper {
    #[debug("handler")]
    handler: Arc<dyn ProtocolHandler>,
}

impl iroh::protocol::ProtocolHandler for ProtocolWrapper {
    async fn accept(
        &self,
        conn: iroh::endpoint::Connection,
    ) -> Result<(), iroh::protocol::AcceptError> {
        self.handler.accept(Arc::new(conn.into())).await
            .map_err(|e| iroh::protocol::AcceptError::from_err(e))?;
        Ok(())
    }

    async fn shutdown(&self) {
        self.handler.shutdown().await;
    }
}

impl Default for NodeOptions {
    fn default() -> Self {
        NodeOptions {
            gc_interval_millis: Some(0),
            blob_events: None,
            enable_docs: false,
            ipv4_addr: None,
            ipv6_addr: None,
            node_discovery: None,
            secret_key: None,
            protocols: None,
        }
    }
}

#[derive(Debug, Default, uniffi::Enum)]
pub enum NodeDiscoveryConfig {
    /// Use no node discovery mechanism.
    None,
    /// Use the default discovery mechanism.
    ///
    /// This uses two discovery services concurrently:
    ///
    /// - It publishes to a pkarr service operated by [number 0] which makes the information
    ///   available via DNS in the `iroh.link` domain.
    ///
    /// - It uses an mDNS-like system to announce itself on the local network.
    ///
    /// [number 0]: https://n0.computer
    #[default]
    Default,
}

/// An Iroh node. Allows you to sync, store, and transfer data.
#[derive(uniffi::Object, Debug, Clone)]
pub struct Iroh {
    pub(crate) router: iroh::protocol::Router,
    pub(crate) store: iroh_blobs::api::Store,
    pub(crate) docs: Option<iroh_docs::api::DocsApi>,
    pub(crate) gossip: Gossip,
    pub(crate) memory_lookup: MemoryLookup,
}

#[uniffi::export]
impl Iroh {
    /// Create a new iroh node.
    ///
    /// The `path` param should be a directory where we can store or load
    /// iroh data from a previous session.
    #[uniffi::constructor(async_runtime = "tokio")]
    pub async fn persistent(path: String) -> Result<Self, IrohError> {
        let options = NodeOptions::default();
        Self::persistent_with_options(path, options).await
    }

    /// Create a new iroh node.
    ///
    /// All data will be only persistet in memory.
    #[uniffi::constructor(async_runtime = "tokio")]
    pub async fn memory() -> Result<Self, IrohError> {
        let options = NodeOptions::default();
        Self::memory_with_options(options).await
    }

    /// Create a new iroh node with options.
    #[uniffi::constructor(async_runtime = "tokio")]
    pub async fn persistent_with_options(
        path: String,
        options: NodeOptions,
    ) -> Result<Self, IrohError> {
        let path = PathBuf::from(path);
        tokio::fs::create_dir_all(&path)
            .await
            .map_err(|err| anyhow::anyhow!(err))?;

        let builder = iroh::Endpoint::builder();
        let (docs_store, author_store) = if options.enable_docs {
            let docs_store = iroh_docs::store::Store::persistent(path.join("docs.redb"))?;
            let author_store =
                iroh_docs::engine::DefaultAuthorStorage::Persistent(path.join("default-author"));

            (Some(docs_store), Some(author_store))
        } else {
            (None, None)
        };
        let blobs_store = iroh_blobs::store::fs::FsStore::load(path.join("blobs"))
            .await
            .map_err(|err| anyhow::anyhow!(err))?;
        let store: iroh_blobs::api::Store = blobs_store.into();

        let (builder, gossip, docs, memory_lookup) = apply_options(
            builder,
            options,
            store.clone(),
            docs_store,
            author_store,
        )
        .await?;
        let router = builder.spawn();

        Ok(Iroh {
            router,
            store,
            docs,
            gossip,
            memory_lookup,
        })
    }

    /// Create a new in memory iroh node with options.
    #[uniffi::constructor(async_runtime = "tokio")]
    pub async fn memory_with_options(options: NodeOptions) -> Result<Self, IrohError> {
        let builder = iroh::Endpoint::builder();

        let (docs_store, author_store) = if options.enable_docs {
            let docs_store = iroh_docs::store::Store::memory();
            let author_store = iroh_docs::engine::DefaultAuthorStorage::Mem;

            (Some(docs_store), Some(author_store))
        } else {
            (None, None)
        };
        let blobs_store = iroh_blobs::store::mem::MemStore::default();
        let store: iroh_blobs::api::Store = blobs_store.into();

        let (builder, gossip, docs, memory_lookup) = apply_options(
            builder,
            options,
            store.clone(),
            docs_store,
            author_store,
        )
        .await?;
        let router = builder.spawn();

        Ok(Iroh {
            router,
            store,
            docs,
            gossip,
            memory_lookup,
        })
    }

    /// Access to node specific functionality.
    pub fn node(&self) -> Node {
        Node { router: self.router.clone() }
    }
}

async fn apply_options(
    mut builder: iroh::endpoint::Builder,
    options: NodeOptions,
    store: iroh_blobs::api::Store,
    docs_store: Option<iroh_docs::store::Store>,
    author_store: Option<iroh_docs::engine::DefaultAuthorStorage>,
) -> anyhow::Result<(
    iroh::protocol::RouterBuilder,
    Gossip,
    Option<iroh_docs::api::DocsApi>,
    MemoryLookup,
)> {
    // Note: gc_period is currently unused - GC is now configured during store creation
    // via GcConfig in the store's Options struct
    let _gc_period = if let Some(millis) = options.gc_interval_millis {
        match millis {
            0 => None,
            millis => Some(Duration::from_millis(millis)),
        }
    } else {
        None
    };

    let blob_events = options.blob_events.map(|cb| BlobProvideEvents::new(cb).into());

    if let Some(addr) = options.ipv4_addr {
        let addr: std::net::SocketAddrV4 = addr.parse()?;
        builder = builder.bind_addr(std::net::SocketAddr::V4(addr))?;
    }

    if let Some(addr) = options.ipv6_addr {
        let addr: std::net::SocketAddrV6 = addr.parse()?;
        builder = builder.bind_addr(std::net::SocketAddr::V6(addr))?;
    }

    // Create a MemoryLookup for out-of-band peer discovery
    let memory_lookup = MemoryLookup::new();

    builder = match options.node_discovery {
        Some(NodeDiscoveryConfig::None) => builder.address_lookup(memory_lookup.clone()),
        Some(NodeDiscoveryConfig::Default) | None => {
            builder
                .address_lookup(DnsAddressLookup::n0_dns())
                .address_lookup(PkarrPublisher::n0_dns())
                .address_lookup(memory_lookup.clone())
        }
    };

    if let Some(secret_key) = options.secret_key {
        let key: [u8; 32] = AsRef::<[u8]>::as_ref(&secret_key).try_into()?;
        let key = iroh::SecretKey::from_bytes(&key);
        builder = builder.secret_key(key);
    }

    let endpoint = builder.bind().await?;
    let mut router_builder = iroh::protocol::Router::builder(endpoint);

    let ffi_endpoint = Arc::new(Endpoint::new(router_builder.endpoint().clone()));

    // Add default protocols for now

    // iroh gossip
    let gossip = Gossip::builder().spawn(router_builder.endpoint().clone());
    router_builder = router_builder.accept(iroh_gossip::ALPN, gossip.clone());

    // iroh blobs
    let blobs = BlobsProtocol::new(&store, blob_events);
    router_builder = router_builder.accept(iroh_blobs::ALPN, blobs);

    let docs = if options.enable_docs {
        let downloader = store.downloader(router_builder.endpoint());
        let engine = iroh_docs::engine::Engine::spawn(
            router_builder.endpoint().clone(),
            gossip.clone(),
            docs_store.expect("docs enabled"),
            store.clone(),
            downloader,
            author_store.expect("docs enabled"),
            None, // protect_cb: Option<ProtectCallbackHandler>
        )
        .await?;
        let docs = Docs::new(engine);
        let api = docs.api().clone();
        router_builder = router_builder.accept(iroh_docs::ALPN, docs);

        Some(api)
    } else {
        None
    };

    // GC is handled by the store itself now via GcConfig during store creation

    // Add custom protocols
    if let Some(protocols) = options.protocols {
        for (alpn, protocol) in protocols {
            let handler = protocol.create(ffi_endpoint.clone());
            router_builder = router_builder.accept(alpn, ProtocolWrapper { handler });
        }
    }

    Ok((router_builder, gossip, docs, memory_lookup))
}

/// Iroh node client.
#[derive(uniffi::Object, Clone)]
pub struct Node {
    router: iroh::protocol::Router,
}

#[uniffi::export]
impl Node {
    /// Shutdown this iroh node.
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn shutdown(&self) -> Result<(), IrohError> {
        self.router.shutdown().await.map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(())
    }

    #[uniffi::method]
    pub fn endpoint(&self) -> Endpoint {
        Endpoint::new(self.router.endpoint().clone())
    }
}

#[derive(Clone)]
struct BlobProvideEvents {
    // TODO: Implement proper event forwarding using the new channel-based EventSender
    #[allow(dead_code)]
    callback: Arc<dyn BlobProvideEventCallback>,
}

impl Debug for BlobProvideEvents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlobProvideEvents()")
    }
}

impl BlobProvideEvents {
    fn new(callback: Arc<dyn BlobProvideEventCallback>) -> Self {
        Self { callback }
    }
}

impl From<BlobProvideEvents> for EventSender {
    fn from(_events: BlobProvideEvents) -> Self {
        // The event system has been completely redesigned in iroh-blobs 0.97
        // The old CustomEventSender trait no longer exists
        // For now, return the default event sender - events callback needs a bigger rewrite
        // TODO: Implement proper event forwarding using the new channel-based EventSender
        EventSender::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory() {
        let node = Iroh::memory().await.unwrap();
        let id = node.node().endpoint().node_id().unwrap();
        println!("{}", id);
    }
}
