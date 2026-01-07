use std::str::FromStr;
use std::sync::Arc;

use crate::blob::{BlobDownloadOptions, BlobFormat, Hash};
use crate::doc::NodeAddr;
use crate::error::IrohError;

// Note: NodeTicket has been removed in iroh 0.95 as tickets are now handled via iroh_tickets
// The main ticket types are now BlobTicket (iroh_blobs::ticket) and DocTicket (iroh_docs)
// EndpointAddr (formerly NodeAddr) can be serialized directly as needed

/// A token containing everything to get a file from the provider.
///
/// It is a single item which can be easily serialized and deserialized.
#[derive(Debug, uniffi::Object)]
#[uniffi::export(Display)]
pub struct BlobTicket(iroh_blobs::ticket::BlobTicket);

impl From<iroh_blobs::ticket::BlobTicket> for BlobTicket {
    fn from(ticket: iroh_blobs::ticket::BlobTicket) -> Self {
        BlobTicket(ticket)
    }
}

impl std::fmt::Display for BlobTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[uniffi::export]
impl BlobTicket {
    #[uniffi::constructor]
    pub fn new(str: String) -> Result<Self, IrohError> {
        let ticket = iroh_blobs::ticket::BlobTicket::from_str(&str).map_err(anyhow::Error::from)?;
        Ok(BlobTicket(ticket))
    }

    /// The hash of the item this ticket can retrieve.
    pub fn hash(&self) -> Arc<Hash> {
        Arc::new(self.0.hash().into())
    }

    /// The [`NodeAddr`] of the provider for this ticket.
    pub fn node_addr(&self) -> Arc<NodeAddr> {
        let addr = self.0.addr().clone();
        Arc::new(addr.into())
    }

    /// The [`BlobFormat`] for this ticket.
    pub fn format(&self) -> BlobFormat {
        self.0.format().into()
    }

    /// True if the ticket is for a collection and should retrieve all blobs in it.
    pub fn recursive(&self) -> bool {
        self.0.format().is_hash_seq()
    }

    /// Convert this ticket into input parameters for a call to blobs_download
    pub fn as_download_options(&self) -> Result<Arc<BlobDownloadOptions>, IrohError> {
        let addr: NodeAddr = self.0.addr().clone().into();
        Ok(Arc::new(BlobDownloadOptions {
            format: self.0.format().into(),
            node: addr,
            tag: crate::blob::SetTagOption::Auto,
        }))
    }
}

/// Options when creating a ticket
#[derive(Debug, uniffi::Enum)]
pub enum AddrInfoOptions {
    /// Only the Node ID is added.
    ///
    /// This usually means that iroh-dns discovery is used to find address information.
    Id,
    /// Include both the relay URL and the direct addresses.
    RelayAndAddresses,
    /// Only include the relay URL.
    Relay,
    /// Only include the direct addresses.
    Addresses,
}

impl From<AddrInfoOptions> for iroh_docs::api::protocol::AddrInfoOptions {
    fn from(options: AddrInfoOptions) -> iroh_docs::api::protocol::AddrInfoOptions {
        match options {
            AddrInfoOptions::Id => iroh_docs::api::protocol::AddrInfoOptions::Id,
            AddrInfoOptions::RelayAndAddresses => {
                iroh_docs::api::protocol::AddrInfoOptions::RelayAndAddresses
            }
            AddrInfoOptions::Relay => iroh_docs::api::protocol::AddrInfoOptions::Relay,
            AddrInfoOptions::Addresses => iroh_docs::api::protocol::AddrInfoOptions::Addresses,
        }
    }
}

/// Contains both a key (either secret or public) to a document, and a list of peers to join.
#[derive(Debug, Clone, uniffi::Object)]
#[uniffi::export(Display)]
pub struct DocTicket(iroh_docs::DocTicket);

impl From<iroh_docs::DocTicket> for DocTicket {
    fn from(value: iroh_docs::DocTicket) -> Self {
        Self(value)
    }
}

impl From<DocTicket> for iroh_docs::DocTicket {
    fn from(value: DocTicket) -> Self {
        value.0
    }
}

#[uniffi::export]
impl DocTicket {
    #[uniffi::constructor]
    pub fn new(str: String) -> Result<Self, IrohError> {
        let ticket = iroh_docs::DocTicket::from_str(&str).map_err(anyhow::Error::from)?;
        Ok(ticket.into())
    }
}

impl std::fmt::Display for DocTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
