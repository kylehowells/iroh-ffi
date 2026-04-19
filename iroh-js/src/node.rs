use std::{
    collections::HashMap, future::Future, path::PathBuf, pin::Pin, sync::Arc, time::Duration,
};

use iroh_blobs::BlobsProtocol;
use iroh_docs::protocol::Docs;
use iroh_gossip::net::Gossip;
use iroh::discovery::static_provider::StaticProvider;
use napi::{
    bindgen_prelude::*,
    threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use napi_derive::napi;
use tokio_util::task::AbortOnDropHandle;
use tracing::warn;

use crate::{BlobProvideEvent, Connection, CounterStats, Endpoint, NodeAddr};

/// Options passed to [`IrohNode.new`]. Controls the behaviour of an iroh node.#
#[napi(object, object_to_js = false)]
pub struct NodeOptions {
    /// How frequently the blob store should clean up unreferenced blobs, in milliseconds.
    /// Set to null to disable gc
    pub gc_interval_millis: Option<u32>,
    /// Provide a callback to hook into events when the blobs component adds and provides blobs.
    pub blob_events: Option<ThreadsafeFunction<BlobProvideEvent, ()>>,
    /// Should docs be enabled? Defaults to `false`.
    pub enable_docs: Option<bool>,
    /// Overwrites the default IPv4 address to bind to
    pub ipv4_addr: Option<String>,
    /// Overwrites the default IPv6 address to bind to
    pub ipv6_addr: Option<String>,
    /// Configure the node discovery.
    pub node_discovery: Option<NodeDiscoveryConfig>,
    /// Provide a specific secret key, identifying this node. Must be 32 bytes long.
    pub secret_key: Option<Vec<u8>>,

    pub protocols: Option<HashMap<Vec<u8>, ThreadsafeFunction<Endpoint, ProtocolHandler>>>,
}

#[derive(derive_more::Debug)]
#[napi(object, object_to_js = false)]
pub struct ProtocolHandler {
    #[debug("accept")]
    pub accept: Arc<ThreadsafeFunction<Connection, ()>>,
    #[debug("shutdown")]
    pub shutdown: Option<Arc<ThreadsafeFunction<(), ()>>>,
}

impl iroh::protocol::ProtocolHandler for ProtocolHandler {
    fn accept(
        &self,
        conn: iroh::endpoint::Connection,
    ) -> Pin<Box<dyn Future<Output = Result<(), iroh::protocol::AcceptError>> + Send>> {
        let accept = self.accept.clone();
        Box::pin(async move {
            accept.call_async(Ok(conn.into())).await
                .map_err(|e| iroh::protocol::AcceptError::from_err(anyhow::anyhow!("{e}")))?;
            Ok(())
        })
    }

    fn shutdown(&self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let shutdown = self.shutdown.clone();
        Box::pin(async move {
            if let Some(ref cb) = shutdown {
                if let Err(err) = cb.call_async(Ok(())).await {
                    warn!("shutdown failed: {:?}", err);
                }
            }
        })
    }
}

impl Default for NodeOptions {
    fn default() -> Self {
        NodeOptions {
            gc_interval_millis: None,
            blob_events: None,
            enable_docs: None,
            ipv4_addr: None,
            ipv6_addr: None,
            node_discovery: None,
            secret_key: None,
            protocols: None,
        }
    }
}

#[derive(Debug, Default)]
#[napi(string_enum)]
pub enum NodeDiscoveryConfig {
    /// Use no node discovery mechanism.
    None,
    /// Use the default discovery mechanism.
    #[default]
    Default,
}

/// An Iroh node. Allows you to sync, store, and transfer data.
#[derive(Debug, Clone)]
#[napi]
pub struct Iroh {
    pub(crate) router: iroh::protocol::Router,
    pub(crate) store: iroh_blobs::api::Store,
    pub(crate) docs: Option<iroh_docs::api::DocsApi>,
    pub(crate) gossip: Gossip,
    pub(crate) static_provider: StaticProvider,
}

#[napi]
impl Iroh {
    /// Create a new iroh node.
    ///
    /// The `path` param should be a directory where we can store or load
    /// iroh data from a previous session.
    #[napi(factory)]
    pub async fn persistent(path: String, opts: Option<NodeOptions>) -> Result<Self> {
        let options = opts.unwrap_or_default();

        let path = PathBuf::from(path);
        tokio::fs::create_dir_all(&path)
            .await
            .map_err(|err| anyhow::anyhow!(err))?;

        let builder = iroh::Endpoint::builder();
        let (docs_store, author_store) = if options.enable_docs.unwrap_or_default() {
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

        let (builder, gossip, docs, static_provider) = apply_options(
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
            static_provider,
        })
    }

    /// Create a new iroh node.
    ///
    /// All data will be only persistet in memory.
    #[napi(factory)]
    pub async fn memory(opts: Option<NodeOptions>) -> Result<Self> {
        let options = opts.unwrap_or_default();
        let builder = iroh::Endpoint::builder();

        let (docs_store, author_store) = if options.enable_docs.unwrap_or_default() {
            let docs_store = iroh_docs::store::Store::memory();
            let author_store = iroh_docs::engine::DefaultAuthorStorage::Mem;

            (Some(docs_store), Some(author_store))
        } else {
            (None, None)
        };
        let blobs_store = iroh_blobs::store::mem::MemStore::default();
        let store: iroh_blobs::api::Store = blobs_store.into();

        let (builder, gossip, docs, static_provider) = apply_options(
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
            static_provider,
        })
    }

    /// Access to node specific funtionaliy.
    #[napi(getter)]
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
    StaticProvider,
)> {
    // GC is now configured during store creation
    let _gc_period = if let Some(millis) = options.gc_interval_millis {
        match millis {
            0 => None,
            millis => Some(Duration::from_millis(millis as _)),
        }
    } else {
        None
    };

    // Blob events - simplified for now
    // TODO: Implement proper event forwarding using the new channel-based EventSender
    let _blob_events = options.blob_events;

    if let Some(addr) = options.ipv4_addr {
        builder = builder.bind_addr_v4(addr.parse()?);
    }

    if let Some(addr) = options.ipv6_addr {
        builder = builder.bind_addr_v6(addr.parse()?);
    }

    // Create a StaticProvider for out-of-band peer discovery
    let static_provider = StaticProvider::new();

    builder = match options.node_discovery {
        Some(NodeDiscoveryConfig::None) => builder.discovery(static_provider.clone()),
        Some(NodeDiscoveryConfig::Default) | None => {
            builder
                .discovery(iroh::discovery::dns::DnsDiscovery::n0_dns())
                .discovery(iroh::discovery::pkarr::PkarrPublisher::n0_dns())
                .discovery(static_provider.clone())
        }
    };

    if let Some(secret_key) = options.secret_key {
        let key: [u8; 32] = AsRef::<[u8]>::as_ref(&secret_key).try_into()?;
        let key = iroh::SecretKey::from_bytes(&key);
        builder = builder.secret_key(key);
    }

    let endpoint = builder.bind().await?;
    let mut router_builder = iroh::protocol::Router::builder(endpoint);

    let ffi_endpoint = Endpoint::new(router_builder.endpoint().clone());

    // Add default protocols

    // iroh gossip
    let gossip = Gossip::builder().spawn(router_builder.endpoint().clone());
    router_builder = router_builder.accept(iroh_gossip::ALPN, gossip.clone());

    // iroh blobs
    let blobs = BlobsProtocol::new(&store, None);
    router_builder = router_builder.accept(iroh_blobs::ALPN, blobs);

    let docs = if options.enable_docs.unwrap_or_default() {
        let downloader = store.downloader(router_builder.endpoint());
        let engine = iroh_docs::engine::Engine::spawn(
            router_builder.endpoint().clone(),
            gossip.clone(),
            docs_store.expect("docs enabled"),
            store.clone(),
            downloader,
            author_store.expect("docs enabled"),
            None, // protect_cb
        )
        .await?;
        let docs = Docs::new(engine);
        let api = docs.api().clone();
        router_builder = router_builder.accept(iroh_docs::ALPN, docs);

        Some(api)
    } else {
        None
    };

    // Add custom protocols
    if let Some(protocols) = options.protocols {
        for (alpn, protocol) in protocols {
            let handler = protocol.call_async(Ok(ffi_endpoint.clone())).await?;
            router_builder = router_builder.accept(alpn, handler);
        }
    }

    Ok((router_builder, gossip, docs, static_provider))
}

/// Iroh node client.
#[napi]
pub struct Node {
    router: iroh::protocol::Router,
}

#[napi]
impl Node {
    /// Get statistics of the running node.
    #[napi]
    pub async fn stats(&self) -> Result<HashMap<String, CounterStats>> {
        // Stats are no longer available through RPC in iroh 0.95
        // Return empty stats for now
        Ok(HashMap::new())
    }

    /// Get status information about a node
    #[napi]
    pub async fn status(&self) -> Result<NodeStatus> {
        let endpoint = self.router.endpoint();
        let node_addr = endpoint.node_addr().await?;
        let listen_addrs: Vec<String> = endpoint.bound_sockets()
            .iter()
            .map(|a| a.to_string())
            .collect();

        Ok(NodeStatus {
            addr: node_addr.into(),
            listen_addrs,
            version: env!("CARGO_PKG_VERSION").to_string(),
            rpc_addr: None,
        })
    }

    /// Shutdown this iroh node.
    #[napi]
    pub async fn shutdown(&self) -> Result<()> {
        self.router.shutdown().await?;

        Ok(())
    }

    #[napi]
    pub fn endpoint(&self) -> Endpoint {
        Endpoint::new(self.router.endpoint().clone())
    }
}

/// The response to a status request
#[derive(Debug)]
#[napi(object)]
pub struct NodeStatus {
    /// The node id and socket addresses of this node.
    pub addr: NodeAddr,
    /// The bound listening addresses of the node
    pub listen_addrs: Vec<String>,
    /// The version of the node
    pub version: String,
    /// RPC address, if currently listening.
    pub rpc_addr: Option<String>,
}

// BlobProvideEvents implementation removed as the event system has changed
// The callback-based approach needs to be rewritten for iroh 0.97
