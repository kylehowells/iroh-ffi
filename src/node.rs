use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

use iroh::address_lookup::{DnsAddressLookup, MemoryLookup, PkarrPublisher};

use crate::{CallbackError, Connection, Endpoint, IrohError};

/// Stats counter.
#[derive(Debug, uniffi::Record)]
pub struct CounterStats {
    /// The counter value.
    pub value: u32,
    /// The counter description.
    pub description: String,
}

/// Options passed to [`IrohNode.new`]. Controls the behaviour of an iroh node.
#[derive(derive_more::Debug, uniffi::Record)]
pub struct NodeOptions {
    /// How frequently the blob store should clean up unreferenced blobs, in milliseconds.
    ///
    /// This is kept for API compatibility, but is currently ignored.
    #[uniffi(default = None)]
    pub gc_interval_millis: Option<u64>,
    /// Should docs be enabled? Defaults to `false`.
    ///
    /// This is currently ignored in the trimmed Synaptic transport build.
    #[uniffi(default = false)]
    pub enable_docs: bool,
    /// Overwrites the default IPv4 address to bind to.
    #[uniffi(default = None)]
    pub ipv4_addr: Option<String>,
    /// Overwrites the default IPv6 address to bind to.
    #[uniffi(default = None)]
    pub ipv6_addr: Option<String>,
    /// Configure the node discovery. Defaults to the default set of config.
    #[uniffi(default = None)]
    pub node_discovery: Option<NodeDiscoveryConfig>,
    /// Provide a specific secret key, identifying this node. Must be 32 bytes long.
    #[uniffi(default = None)]
    pub secret_key: Option<Vec<u8>>,
    /// Additional transport bias overrides applied during path selection.
    ///
    /// This currently supports tuning IPv4 and IPv6 priority relative to each other.
    #[uniffi(default = None)]
    pub transport_biases: Option<Vec<NodeTransportBiasConfig>>,
    #[uniffi(default = None)]
    pub protocols: Option<HashMap<Vec<u8>, Arc<dyn ProtocolCreator>>>,
}

/// Transport classes that can be biased during path selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum NodeTransportKind {
    /// IPv4 UDP transport paths.
    Ipv4,
    /// IPv6 UDP transport paths.
    Ipv6,
    /// Relay-backed transport paths.
    Relay,
}

/// Additional transport bias to apply while building the endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct NodeTransportBiasConfig {
    /// The transport class to bias.
    pub kind: NodeTransportKind,
    /// Additional RTT advantage, in milliseconds.
    ///
    /// Larger values make this transport class more preferred during path selection.
    #[uniffi(default = 0)]
    pub rtt_advantage_millis: u64,
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
        self.handler
            .accept(Arc::new(conn.into()))
            .await
            .map_err(iroh::protocol::AcceptError::from_err)?;
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
            enable_docs: false,
            ipv4_addr: None,
            ipv6_addr: None,
            node_discovery: None,
            secret_key: None,
            transport_biases: None,
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

/// An Iroh node. Allows you to manage transport state and endpoint connectivity.
#[derive(uniffi::Object, Debug, Clone)]
pub struct Iroh {
    pub(crate) router: iroh::protocol::Router,
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
    /// All data will be only persisted in memory.
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

        let builder = iroh::Endpoint::builder(iroh::endpoint::presets::N0);
        let (builder, memory_lookup) = apply_options(builder, options).await?;
        let router = builder.spawn();

        Ok(Iroh {
            router,
            memory_lookup,
        })
    }

    /// Create a new in-memory iroh node with options.
    #[uniffi::constructor(async_runtime = "tokio")]
    pub async fn memory_with_options(options: NodeOptions) -> Result<Self, IrohError> {
        let builder = iroh::Endpoint::builder(iroh::endpoint::presets::N0);
        let (builder, memory_lookup) = apply_options(builder, options).await?;
        let router = builder.spawn();

        Ok(Iroh {
            router,
            memory_lookup,
        })
    }

    /// Access node-specific functionality.
    pub fn node(&self) -> Node {
        Node {
            router: self.router.clone(),
        }
    }
}

async fn apply_options(
    mut builder: iroh::endpoint::Builder,
    options: NodeOptions,
) -> anyhow::Result<(iroh::protocol::RouterBuilder, MemoryLookup)> {
    if let Some(addr) = options.ipv4_addr {
        let addr: std::net::SocketAddrV4 = addr.parse()?;
        builder = builder.bind_addr(std::net::SocketAddr::V4(addr))?;
    }

    if let Some(addr) = options.ipv6_addr {
        let addr: std::net::SocketAddrV6 = addr.parse()?;
        builder = builder.bind_addr(std::net::SocketAddr::V6(addr))?;
    }

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

    if let Some(transport_biases) = options.transport_biases {
        for transport_bias in transport_biases {
            let kind = match transport_bias.kind {
                NodeTransportKind::Ipv4 => iroh::endpoint::transports::AddrKind::IpV4,
                NodeTransportKind::Ipv6 => iroh::endpoint::transports::AddrKind::IpV6,
                NodeTransportKind::Relay => iroh::endpoint::transports::AddrKind::Relay,
            };
            let bias = iroh::endpoint::transports::TransportBias::primary()
                .with_rtt_advantage(Duration::from_millis(transport_bias.rtt_advantage_millis));
            builder = builder.transport_bias(kind, bias);
        }
    }

    let endpoint = builder.bind().await?;
    let mut router_builder = iroh::protocol::Router::builder(endpoint);

    let ffi_endpoint = Arc::new(Endpoint::new(router_builder.endpoint().clone()));

    if let Some(protocols) = options.protocols {
        for (alpn, protocol) in protocols {
            let handler = protocol.create(ffi_endpoint.clone());
            router_builder = router_builder.accept(alpn, ProtocolWrapper { handler });
        }
    }

    Ok((router_builder, memory_lookup))
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
        self.router
            .shutdown()
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(())
    }

    #[uniffi::method]
    pub fn endpoint(&self) -> Endpoint {
        Endpoint::new(self.router.endpoint().clone())
    }
}
