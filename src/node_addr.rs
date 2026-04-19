use std::{str::FromStr, sync::Arc};

use crate::PublicKey;

/// A peer and its addressing information.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Object)]
pub struct NodeAddr {
    node_id: Arc<PublicKey>,
    relay_url: Option<String>,
    addresses: Vec<String>,
}

#[uniffi::export]
impl NodeAddr {
    /// Create a new [`NodeAddr`] with explicit relay and direct address candidates.
    #[uniffi::constructor]
    pub fn new(node_id: &PublicKey, derp_url: Option<String>, addresses: Vec<String>) -> Self {
        Self {
            node_id: Arc::new(node_id.clone()),
            relay_url: derp_url,
            addresses,
        }
    }

    /// Get the direct addresses of this peer.
    pub fn direct_addresses(&self) -> Vec<String> {
        self.addresses.clone()
    }

    /// Get the home relay URL for this peer.
    pub fn relay_url(&self) -> Option<String> {
        self.relay_url.clone()
    }

    /// Returns true if both node addresses have the same values.
    pub fn equal(&self, other: &NodeAddr) -> bool {
        self == other
    }
}

impl TryFrom<NodeAddr> for iroh::EndpointAddr {
    type Error = anyhow::Error;

    fn try_from(value: NodeAddr) -> Result<Self, Self::Error> {
        let mut endpoint_addr = iroh::EndpointAddr::new((&*value.node_id).into());
        let addresses = value
            .direct_addresses()
            .into_iter()
            .map(|addr| std::net::SocketAddr::from_str(&addr))
            .collect::<Result<Vec<_>, _>>()?;

        if let Some(derp_url) = value.relay_url() {
            let url = url::Url::parse(&derp_url)?;
            endpoint_addr = endpoint_addr.with_relay_url(url.into());
        }
        for addr in addresses {
            endpoint_addr = endpoint_addr.with_ip_addr(addr);
        }
        Ok(endpoint_addr)
    }
}

impl From<iroh::EndpointAddr> for NodeAddr {
    fn from(value: iroh::EndpointAddr) -> Self {
        NodeAddr {
            node_id: Arc::new(value.id.into()),
            relay_url: value.relay_urls().next().map(|url| url.to_string()),
            addresses: value.ip_addrs().map(|addr| addr.to_string()).collect(),
        }
    }
}
