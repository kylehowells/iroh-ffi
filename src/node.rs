use std::{collections::HashMap, fmt::Debug, path::PathBuf, sync::Arc, time::Duration};

use iroh_blobs::{
    BlobsProtocol,
    provider::events::EventSender,
};
use iroh_docs::protocol::Docs;
use iroh_gossip::net::Gossip;
use iroh::discovery::static_provider::StaticProvider;

use crate::{
    BlobProvideEventCallback, CallbackError, Connection, Endpoint, IrohError, PublicKey,
};

/// Stats counter
#[derive(Debug, uniffi::Record)]
pub struct CounterStats {
    /// The counter value
    pub value: u32,
    /// The counter description
    pub description: String,
}

/// Information about a direct address.
#[derive(Debug, Clone, uniffi::Object)]
pub struct DirectAddrInfo(pub(crate) iroh::endpoint::DirectAddrInfo);

#[uniffi::export]
impl DirectAddrInfo {
    /// Get the reported address
    pub fn addr(&self) -> String {
        self.0.addr.to_string()
    }

    /// Get the reported latency, if it exists
    pub fn latency(&self) -> Option<Duration> {
        self.0.latency
    }

    /// Get the last control message received by this node
    pub fn last_control(&self) -> Option<LatencyAndControlMsg> {
        self.0
            .last_control
            .map(|(latency, control_msg)| LatencyAndControlMsg {
                latency,
                control_msg: control_msg.to_string(),
            })
    }

    /// Get how long ago the last payload message was received for this node
    pub fn last_payload(&self) -> Option<Duration> {
        self.0.last_payload
    }
}

/// The latency and type of the control message
#[derive(Debug, uniffi::Record)]
pub struct LatencyAndControlMsg {
    /// The latency of the control message
    pub latency: Duration,
    /// The type of control message, represented as a string
    pub control_msg: String,
    // control_msg: ControlMsg
}

// TODO: enable and use for `LatencyAndControlMsg.control_msg` field when iroh core makes this public
// The kinds of control messages that can be sent
// pub use iroh::magicsock::ControlMsg;

/// Information about a remote node
#[derive(Debug, uniffi::Record)]
pub struct RemoteInfo {
    /// The node identifier of the endpoint. Also a public key.
    pub node_id: Arc<PublicKey>,
    /// Relay url, if available.
    pub relay_url: Option<String>,
    /// List of addresses at which this node might be reachable, plus any latency information we
    /// have about that address and the last time the address was used.
    pub addrs: Vec<Arc<DirectAddrInfo>>,
    /// The type of connection we have to the peer, either direct or over relay.
    pub conn_type: Arc<ConnectionType>,
    /// The latency of the `conn_type`.
    pub latency: Option<Duration>,
    /// Duration since the last time this peer was used.
    pub last_used: Option<Duration>,
}

// RemoteInfo has been removed in iroh 0.93+, keeping struct for FFI compatibility
// but removing the From impl since iroh::endpoint::RemoteInfo no longer exists

/// The type of the connection
#[derive(Debug, uniffi::Enum)]
pub enum ConnType {
    /// Indicates you have a UDP connection.
    Direct,
    /// Indicates you have a relayed connection.
    Relay,
    /// Indicates you have an unverified UDP connection, and a relay connection for backup.
    Mixed,
    /// Indicates you have no proof of connection.
    None,
}

/// The type of connection we have to the node
#[derive(Debug, uniffi::Object)]
pub enum ConnectionType {
    /// Direct UDP connection
    Direct(String),
    /// Relay connection
    Relay(String),
    /// Both a UDP and a Relay connection are used.
    ///
    /// This is the case if we do have a UDP address, but are missing a recent confirmation that
    /// the address works.
    Mixed(String, String),
    /// We have no verified connection to this PublicKey
    None,
}

#[uniffi::export]
impl ConnectionType {
    /// Whether connection is direct, relay, mixed, or none
    pub fn r#type(&self) -> ConnType {
        match self {
            ConnectionType::Direct(_) => ConnType::Direct,
            ConnectionType::Relay(_) => ConnType::Relay,
            ConnectionType::Mixed(..) => ConnType::Mixed,
            ConnectionType::None => ConnType::None,
        }
    }

    /// Return the socket address if this is a direct connection
    pub fn as_direct(&self) -> String {
        match self {
            ConnectionType::Direct(addr) => addr.clone(),
            _ => panic!("ConnectionType type is not 'Direct'"),
        }
    }

    /// Return the derp url if this is a relay connection
    pub fn as_relay(&self) -> String {
        match self {
            ConnectionType::Relay(url) => url.clone(),
            _ => panic!("ConnectionType is not `Relay`"),
        }
    }

    /// Return the socket address and DERP url if this is a mixed connection
    pub fn as_mixed(&self) -> ConnectionTypeMixed {
        match self {
            ConnectionType::Mixed(addr, url) => ConnectionTypeMixed {
                addr: addr.clone(),
                relay_url: url.clone(),
            },
            _ => panic!("ConnectionType is not `Relay`"),
        }
    }
}

/// The socket address and url of the mixed connection
#[derive(Debug, uniffi::Record)]
pub struct ConnectionTypeMixed {
    /// Address of the node
    pub addr: String,
    /// Url of the relay node to which the node is connected
    pub relay_url: String,
}

impl From<iroh::endpoint::ConnectionType> for ConnectionType {
    fn from(value: iroh::endpoint::ConnectionType) -> Self {
        match value {
            iroh::endpoint::ConnectionType::Direct(addr) => {
                ConnectionType::Direct(addr.to_string())
            }
            iroh::endpoint::ConnectionType::Mixed(addr, url) => {
                ConnectionType::Mixed(addr.to_string(), url.to_string())
            }
            iroh::endpoint::ConnectionType::Relay(url) => ConnectionType::Relay(url.to_string()),
            iroh::endpoint::ConnectionType::None => ConnectionType::None,
        }
    }
}
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
    /// # Usage during tests
    ///
    /// Note that the default changes when compiling with `cfg(test)` or the `test-utils`
    /// cargo feature from [iroh-net] is enabled.  In this case only the Pkarr/DNS service
    /// is used, but on the `iroh.test` domain.  This domain is not integrated with the
    /// global DNS network and thus node discovery is effectively disabled.  To use node
    /// discovery in a test use the [`iroh_net::test_utils::DnsPkarrServer`] in the test and
    /// configure it here as a custom discovery mechanism ([`DiscoveryConfig::Custom`]).
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
    pub(crate) static_provider: StaticProvider,
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
    StaticProvider,
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

    Ok((router_builder, gossip, docs, static_provider))
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

// NodeStatus removed - was based on iroh_node_util which no longer exists
// Status information can be obtained directly from the Endpoint

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
