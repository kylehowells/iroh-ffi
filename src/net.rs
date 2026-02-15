use std::sync::Arc;

use iroh::address_lookup::MemoryLookup;

use crate::{Iroh, IrohError, NodeAddr};

/// Iroh net client.
#[derive(uniffi::Object, Clone)]
pub struct Net {
    endpoint: iroh::Endpoint,
    memory_lookup: MemoryLookup,
}

#[uniffi::export]
impl Iroh {
    /// Access to network specific functionality.
    pub fn net(&self) -> Net {
        Net {
            endpoint: self.router.endpoint().clone(),
            memory_lookup: self.memory_lookup.clone(),
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

    // Note: latency() has been removed in iroh 0.96.
    // Use Connection::rtt() for per-connection latency instead.

    /// Add endpoint addressing information for out-of-band peer discovery.
    ///
    /// This is used to inform the node about peer addresses obtained through
    /// some out-of-band mechanism (e.g., exchanged via gossip topic subscription,
    /// QR codes, tickets, etc.). The MemoryLookup will use this information
    /// to help establish connections to the given peer.
    pub fn add_node_addr(&self, node_addr: Arc<NodeAddr>) -> Result<(), IrohError> {
        let endpoint_addr: iroh::EndpointAddr = (*node_addr).clone().try_into()?;
        self.memory_lookup.add_endpoint_info(endpoint_addr);
        Ok(())
    }
}
