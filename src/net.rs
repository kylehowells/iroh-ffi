use std::sync::Arc;
use std::time::Duration;

use iroh::discovery::static_provider::StaticProvider;

use crate::{Iroh, IrohError, NodeAddr, PublicKey};

/// Iroh net client.
#[derive(uniffi::Object, Clone)]
pub struct Net {
    endpoint: iroh::Endpoint,
    static_provider: StaticProvider,
}

#[uniffi::export]
impl Iroh {
    /// Access to network specific functionality.
    pub fn net(&self) -> Net {
        Net {
            endpoint: self.router.endpoint().clone(),
            static_provider: self.static_provider.clone(),
        }
    }
}

#[uniffi::export]
impl Net {
    /// The string representation of the PublicKey of this node.
    pub fn node_id(&self) -> String {
        self.endpoint.id().to_string()
    }

    /// Return the [`NodeAddr`] for this node.
    pub fn node_addr(&self) -> NodeAddr {
        self.endpoint.addr().into()
    }

    /// Wait for the endpoint to be online (connected to relay and has direct addresses).
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn wait_online(&self) -> Result<(), IrohError> {
        self.endpoint.online().await;
        Ok(())
    }

    /// Get the latency to a specific node, if we have connection info for it.
    pub fn latency(&self, node_id: &PublicKey) -> Option<Duration> {
        let id: iroh::PublicKey = node_id.into();
        self.endpoint.latency(id)
    }

    /// Add endpoint addressing information for out-of-band peer discovery.
    ///
    /// This is used to inform the node about peer addresses obtained through
    /// some out-of-band mechanism (e.g., exchanged via gossip topic subscription,
    /// QR codes, tickets, etc.). The StaticProvider will use this information
    /// to help establish connections to the given peer.
    pub fn add_node_addr(&self, node_addr: Arc<NodeAddr>) -> Result<(), IrohError> {
        let endpoint_addr: iroh::EndpointAddr = (*node_addr).clone().try_into()?;
        self.static_provider.add_endpoint_info(endpoint_addr);
        Ok(())
    }
}
