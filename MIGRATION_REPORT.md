# Iroh FFI Migration Report: 0.35 → 0.95

**Date:** January 7, 2026
**Current Version:** 0.35.0
**Target Version:** 0.95.x

## Executive Summary

Migrating the iroh-ffi bindings from version 0.35 to 0.95 requires a **fundamental architectural rewrite**, not merely API updates. The iroh ecosystem underwent major restructuring between these versions:

- The RPC-based architecture (`quic-rpc`) was replaced with a direct API (`irpc`)
- The `iroh-node-util` crate was removed entirely
- Crate responsibilities were reorganized (blobs, docs, gossip split into separate repos)
- Core terminology changed (`Node` → `Endpoint`)
- Multiple types and modules were removed, renamed, or relocated

**Compilation Status:** 187 errors after dependency update
**Estimated Effort:** Significant rewrite of core modules

---

## Table of Contents

1. [Version Compatibility Matrix](#version-compatibility-matrix)
2. [Architectural Changes](#architectural-changes)
3. [Breaking Changes by Release](#breaking-changes-by-release)
4. [File-by-File Migration Analysis](#file-by-file-migration-analysis)
5. [Removed APIs](#removed-apis)
6. [New APIs to Adopt](#new-apis-to-adopt)
7. [Migration Strategy](#migration-strategy)
8. [Risk Assessment](#risk-assessment)

---

## Version Compatibility Matrix

| Crate | Old Version | New Version | Notes |
|-------|-------------|-------------|-------|
| `iroh` | 0.35 | 0.95 | Core networking, Node→Endpoint rename |
| `iroh-base` | 0.35 | 0.95 | Ticket module removed |
| `iroh-blobs` | 0.35 | 0.97 | Complete API redesign, separate repo |
| `iroh-docs` | 0.35 | 0.95 | RPC removed, separate repo |
| `iroh-gossip` | 0.35 | 0.95 | Separate repo |
| `iroh-metrics` | 0.34 | 0.37 | Minor updates |
| `iroh-node-util` | 0.35 | **REMOVED** | Crate deleted |
| `quic-rpc` | 0.20 | **REMOVED** | Replaced by `irpc` |
| `iroh-tickets` | N/A | 0.2 | **NEW** - tickets extracted |
| `n0-future` | N/A | 0.3.0 | **NEW** - async utilities |
| `n0-snafu` | N/A | 0.2 | **NEW** - error handling |

### Rust Version Requirement

- **Old:** Rust 1.83
- **New:** Rust 1.85, Edition 2024

---

## Architectural Changes

### 1. RPC Layer Removal

**Before (0.35):** The FFI used a two-tier RPC architecture:
```
FFI Layer → quic-rpc Client → RPC Server → Internal Implementation
```

```rust
// Old pattern: RPC-based clients
use quic_rpc::{transport::flume::FlumeConnector, RpcClient, RpcServer};
use iroh_node_util::rpc::client::net::Client;

type BlobsClient = iroh_blobs::rpc::client::blobs::Client<
    FlumeConnector<iroh_blobs::rpc::proto::Response, iroh_blobs::rpc::proto::Request>,
>;
```

**After (0.95):** Direct API access:
```rust
// New pattern: Direct API
use iroh_blobs::api::{Store, Blobs};

let store = Store::load(path).await?;
let blobs = Blobs::new(store, endpoint);
```

### 2. Crate Reorganization

```
iroh (monorepo 0.35)           iroh (0.95+)
├── iroh                       ├── iroh (networking only)
├── iroh-base                  ├── iroh-base
├── iroh-blobs ─────────────── │   (separate repo: n0-computer/iroh-blobs)
├── iroh-docs ──────────────── │   (separate repo: n0-computer/iroh-docs)
├── iroh-gossip ────────────── │   (separate repo: n0-computer/iroh-gossip)
├── iroh-node-util ─────────── │   DELETED
└── iroh-relay                 └── iroh-relay
```

### 3. Store Architecture (iroh-blobs)

**Before:** Two-tier store trait hierarchy with RPC layer
```rust
// Store implemented traits, RPC provided client interface
impl iroh_blobs::store::Store for MyStore { ... }
let client = iroh_blobs::rpc::client::blobs::Client::new(...);
```

**After:** Unified API with `irpc` internally
```rust
// Single Store type with direct methods
use iroh_blobs::api::Store;
let store = Store::load(path).await?;
let hash = store.import_file(path).await?;
```

### 4. Progress/Streaming Pattern

**Before:** Streams returned directly
```rust
let stream = client.download(hash).await?;
while let Some(progress) = stream.next().await { ... }
```

**After:** Progress structs with `IntoFuture` and `.stream()`:
```rust
let progress = store.download(hash);
// Simple: just await
let result = progress.await?;
// Or stream events:
let mut stream = progress.stream();
while let Some(event) = stream.next().await { ... }
```

---

## Breaking Changes by Release

### iroh-blobs 0.90 (Major Rewrite)

| Change | Impact |
|--------|--------|
| `quic-rpc` → `irpc` | All RPC client code must be rewritten |
| `RangeSpec` → `ChunkRanges` | Type renames throughout |
| `anyhow` → `snafu` for errors | Error handling patterns change |
| Store trait hierarchy flattened | Store implementation simplified |
| New `GetMany`, `Push`, `Observe` requests | New capabilities available |
| Bitfield tracking for partial blobs | Storage format change (forward compatible) |

**Source:** https://www.iroh.computer/blog/iroh-blobs-0-90-changes

### iroh 0.91 (Relay Protocol Break)

| Change | Impact |
|--------|--------|
| Relay wire protocol changed | Network incompatibility with <0.91 |
| `Watcher::get()` now requires `&mut self` | Clone watcher before calling |
| `Builder::relay_conn_protocol` removed | Always websockets now |
| `ClientToRelayMsg::SendPacket` removed | Use `Datagrams` instead |

**Source:** https://www.iroh.computer/blog/iroh-0-91-0-the-last-relay-break

### iroh 0.92 (mDNS Changes)

| Change | Impact |
|--------|--------|
| `MdnsDiscovery::new(node_id)` → `new(node_id, advertise)` | Constructor signature change |
| `subscribe()` returns `DiscoveryEvent` not `DiscoveryItem` | Must handle `Discovered`/`Expired` variants |

**Source:** https://www.iroh.computer/blog/iroh-0-92-0-mdns-improvements

### iroh 0.93 (API Cleanup)

| Removed | Replacement |
|---------|-------------|
| `Endpoint::direct_addresses()` | `watch_node_addr()` |
| `Endpoint::home_relay()` | `online()` then `node_addr()` |
| `Endpoint::add_node_addr()` | `StaticProvider` discovery |
| `Endpoint::remote_info()` | `latency()` method |
| `RemoteInfo` type | Removed entirely |
| `DiscoveryItem` | Part of `DiscoveryEvent` |

| New API | Purpose |
|---------|---------|
| `Endpoint::online()` | Wait for relay + direct addresses |
| `Endpoint::watch_node_addr()` | Returns `Watcher<NodeAddr>` |
| `Endpoint::latency(NodeId)` | Get connection latency |

| Changed | Details |
|---------|---------|
| `PublicKey::fmt_short()` | Returns `impl Display` not `String` |
| `MdnsDiscovery::new()` | Now private, use `builder()` |

**Source:** https://www.iroh.computer/blog/iroh-0-93-iroh-online

### iroh 0.94 (Node → Endpoint Rename)

**Major terminology shift affecting all code:**

| Old | New |
|-----|-----|
| `NodeAddr` | `EndpointAddr` |
| `NodeId` | `EndpointId` |
| `NodeTicket` | `EndpointTicket` |
| `RelayNode` | `RelayConfig` |
| `Endpoint::node_id()` | `Endpoint::id()` |
| `Endpoint::node_addr()` | `Endpoint::addr()` |
| `Endpoint::watch_node_addr()` | `Endpoint::watch_addr()` |

**Address structure overhaul:**
```rust
// Old: Two separate fields
direct_addresses: BTreeSet<SocketAddr>,
relay_url: Option<RelayUrl>,

// New: Unified TransportAddr enum
addrs: BTreeSet<TransportAddr>,

pub enum TransportAddr {
    Relay(RelayUrl),
    Ip(SocketAddr),
}
```

**Tickets moved:**
```rust
// Old
use iroh_base::ticket::NodeTicket;

// New
use iroh_tickets::NodeTicket;  // Note: may still use old name
```

**Source:** https://www.iroh.computer/blog/iroh-0-94-0-the-endpoint-takeover

### iroh 0.95 (Error Handling)

| Change | Impact |
|--------|--------|
| `snafu` → `n0-error` | Error types changed again |
| `Connection::remote_id()` | Now infallible (no Result) |
| `Connection::alpn()` | Now infallible (no Result) |
| `ProtocolHandler::on_connecting()` | Replaced by `on_accepting()` |
| `IncomingFuture` | Use `Accepting` instead |
| 0-RTT API restructured | New `OutgoingZeroRttConnection` type |

**Source:** https://www.iroh.computer/blog/iroh-0-95-0-new-relay

---

## File-by-File Migration Analysis

### `src/node.rs` - **COMPLETE REWRITE REQUIRED**

**Current state:** 737 lines, core of the FFI
**Error count:** ~50+ errors
**Severity:** Critical

**Issues:**
1. `iroh_node_util` imports - crate removed
2. `quic_rpc` imports - crate removed
3. `iroh_blobs::net_protocol::Blobs` - module now private
4. `iroh_blobs::provider::EventSender` - now private
5. `iroh_blobs::store::fs::Store` - now private, use `api::Store`
6. `iroh_blobs::store::mem::Store` - now private
7. `iroh::endpoint::RemoteInfo` - type removed
8. `iroh::endpoint::ConnectionType` - may have changed
9. RPC client types (`BlobsClient`, `DocsClient`, etc.) - pattern removed
10. `ProtocolHandler` trait - return type changed to `AcceptError`

**Required changes:**
```rust
// Remove these imports entirely
use iroh_node_util::rpc::server::AbstractNode;
use quic_rpc::{transport::flume::FlumeConnector, RpcClient, RpcServer};

// Replace store creation
// Old:
let blobs_store = iroh_blobs::store::fs::Store::load(path).await?;
// New:
let store = iroh_blobs::api::Store::load(path).await?;

// Replace Blobs setup
// Old:
let blobs = Blobs::builder(blob_store).events(events).build(endpoint);
// New:
let blobs = iroh_blobs::Blobs::new(store.clone(), endpoint.clone());

// Update ProtocolHandler impl
impl iroh::protocol::ProtocolHandler for ProtocolWrapper {
    fn accept(&self, conn: Connection) -> impl Future<Output = Result<(), AcceptError>> + Send {
        // ...
    }
}
```

**The `Iroh` struct must be redesigned** - it currently holds RPC clients that no longer exist.

### `src/net.rs` - **COMPLETE REWRITE REQUIRED**

**Current state:** 74 lines
**Error count:** ~10 errors
**Severity:** Critical

**Issues:**
1. `iroh_node_util::rpc::client::net::Client` - crate removed
2. `remote_info_iter()` - method removed
3. `remote_info()` - method removed
4. `home_relay()` - method removed (use `online()` + `addr()`)
5. `add_node_addr()` - removed (use `StaticProvider`)

**Required changes:**
The entire `Net` client concept needs rethinking. Methods should call `Endpoint` directly:

```rust
// Old
impl Net {
    pub async fn node_id(&self) -> Result<String, IrohError> {
        let id = self.client.node_id().await?;
        Ok(id.to_string())
    }
}

// New - use Endpoint directly
impl Net {
    pub fn node_id(&self) -> Result<String, IrohError> {
        Ok(self.endpoint.id().to_string())
    }
}
```

### `src/blob.rs` - **MAJOR CHANGES REQUIRED**

**Current state:** ~1700 lines (estimated)
**Error count:** ~40+ errors
**Severity:** High

**Issues:**
1. `iroh_blobs::rpc::client::blobs::*` - removed
2. `BlobDownloadOptions` - may have changed
3. Download/upload progress patterns changed
4. `SetTagOption` moved
5. `WrapOption` may have changed

**Required changes:**
```rust
// Old: RPC client pattern
pub struct Blobs {
    client: BlobsClient,
}

impl Blobs {
    pub async fn list(&self) -> Result<Vec<Arc<Hash>>, IrohError> {
        let response = self.client.list().await?;
        // ...
    }
}

// New: Direct API pattern
pub struct Blobs {
    store: iroh_blobs::api::Store,
}

impl Blobs {
    pub async fn list(&self) -> Result<Vec<Arc<Hash>>, IrohError> {
        let hashes = self.store.blobs().list().await?;
        // ...
    }
}
```

### `src/doc.rs` - **MAJOR CHANGES REQUIRED**

**Current state:** ~800 lines (estimated)
**Error count:** ~30+ errors
**Severity:** High

**Issues:**
1. `iroh_docs::rpc::client::docs::*` - removed
2. `iroh_docs::rpc::AddrInfoOptions` - moved to `api::protocol`
3. Document/Author client patterns changed

### `src/tag.rs` - **MODERATE CHANGES**

**Current state:** ~100 lines
**Error count:** ~5 errors
**Severity:** Medium

**Issues:**
1. `iroh_blobs::rpc::client::tags::TagInfo` - path changed
2. `iroh_blobs::Tag` - path changed to `api::Tag`

```rust
// Old
use iroh_blobs::rpc::client::tags::TagInfo;
let tag = iroh_blobs::Tag(Bytes::from(name));

// New
use iroh_blobs::api::tags::TagInfo;
use iroh_blobs::api::Tag;
let tag = Tag::from(name);
```

### `src/ticket.rs` - **MODERATE CHANGES**

**Current state:** ~150 lines
**Error count:** ~10 errors
**Severity:** Medium

**Issues:**
1. `iroh_base::ticket::NodeTicket` - moved to `iroh_tickets`
2. `iroh::NodeAddr` - renamed to `EndpointAddr`
3. `iroh_blobs::util::SetTagOption` - path changed
4. `iroh_blobs::net_protocol::DownloadMode` - module private
5. `iroh_docs::rpc::AddrInfoOptions` - path changed

```rust
// Old
use iroh_base::ticket::NodeTicket;

// New
use iroh_tickets::NodeTicket;
```

### `src/endpoint.rs` - **MINOR CHANGES**

**Current state:** 348 lines
**Error count:** ~5 errors
**Severity:** Low-Medium

**Issues:**
1. `NodeAddr` → `EndpointAddr` in some places
2. `node_id()` → `id()`
3. `remote_node_id()` - verify still exists
4. `Connection::alpn()` - now returns `Vec<u8>` directly (was `Option`)

```rust
// Old
pub fn node_id(&self) -> Result<String, IrohError> {
    let id = self.0.node_id();
    Ok(id.to_string())
}

// New
pub fn id(&self) -> Result<String, IrohError> {
    let id = self.0.id();
    Ok(id.to_string())
}
```

### `src/key.rs` - **MINOR CHANGES**

**Current state:** 111 lines
**Error count:** ~2 errors
**Severity:** Low

**Issues:**
1. `PublicKey::fmt_short()` returns `impl Display` not `String`

```rust
// Old
pub fn fmt_short(&self) -> String {
    iroh::PublicKey::from(self).fmt_short()
}

// New
pub fn fmt_short(&self) -> String {
    iroh::PublicKey::from(self).fmt_short().to_string()
}
```

### `src/error.rs` - **MINOR CHANGES**

**Current state:** ~30 lines
**Error count:** ~1 error
**Severity:** Low

The error wrapper should still work, but consider adopting `n0-error` patterns.

### `src/author.rs` - **MODERATE CHANGES**

**Severity:** Medium
Depends on `iroh_docs` API changes.

### `src/gossip.rs` - **MODERATE CHANGES**

**Severity:** Medium
`iroh_gossip` API may have changed.

### `src/lib.rs` - **MINOR CHANGES**

**Current state:** 151 lines
**Error count:** ~2 errors
**Severity:** Low

**Issues:**
1. `iroh_blobs::util::fs::key_to_path` - verify path still exists
2. `iroh_blobs::util::fs::path_to_key` - verify path still exists

---

## Removed APIs

### Completely Removed Crates
- `iroh-node-util` - All functionality must be reimplemented or removed

### Removed Types
| Type | Was In | Notes |
|------|--------|-------|
| `RemoteInfo` | `iroh::endpoint` | Use `latency()` instead |
| `DiscoveryItem` | `iroh::discovery` | Now inside `DiscoveryEvent` |

### Removed Methods
| Method | Was On | Replacement |
|--------|--------|-------------|
| `direct_addresses()` | `Endpoint` | `watch_addr()` |
| `home_relay()` | `Endpoint` | `online()` + `addr()` |
| `add_node_addr()` | `Endpoint` | `StaticProvider` |
| `remote_info()` | `Endpoint` | `latency()` |
| `remote_info_iter()` | Net client | Removed |
| `on_connecting()` | `ProtocolHandler` | `on_accepting()` |

### Removed Modules (Made Private)
| Module | Crate | Notes |
|--------|-------|-------|
| `rpc` | `iroh_blobs` | Use `api` module |
| `rpc` | `iroh_docs` | Use `api` module |
| `net_protocol` | `iroh_blobs` | Internal only |
| `store::fs` | `iroh_blobs` | Use `api::Store` |
| `store::mem` | `iroh_blobs` | Use `api::Store` |
| `provider::EventSender` | `iroh_blobs` | Internal only |

---

## New APIs to Adopt

### iroh-blobs 0.97

```rust
// New unified store API
use iroh_blobs::api::{Store, Blobs, Tag, tags::TagInfo};

// Create store
let store = Store::load(path).await?;  // Persistent
let store = Store::memory();            // In-memory

// Blob operations via store
let hash = store.import_file(path).await?;
let hash = store.import_bytes(data).await?;
store.export_file(hash, dest_path).await?;

// Progress pattern
let progress = store.download(hash, opts);
progress.await?;  // Simple
// or
let mut stream = progress.stream();
while let Some(event) = stream.next().await { ... }
```

### iroh 0.95

```rust
// Endpoint (was Node)
let endpoint = Endpoint::builder().bind().await?;
let id = endpoint.id();  // was node_id()
let addr = endpoint.addr();  // was node_addr()

// Wait for connectivity
endpoint.online().await?;

// Connection latency
if let Some(latency) = endpoint.latency(peer_id) {
    println!("Latency: {:?}", latency);
}

// Address structure
let addr: EndpointAddr = endpoint.addr();
for transport_addr in addr.addrs() {
    match transport_addr {
        TransportAddr::Ip(socket) => println!("Direct: {}", socket),
        TransportAddr::Relay(url) => println!("Relay: {}", url),
    }
}
```

### iroh-tickets 0.2

```rust
use iroh_tickets::{NodeTicket, BlobTicket, DocTicket};

let ticket = NodeTicket::new(endpoint_addr);
let ticket_str = ticket.to_string();
let ticket = NodeTicket::from_str(&ticket_str)?;
```

---

## Migration Strategy

### Phase 1: Foundation (Estimated: 2-3 days)

1. **Update `Cargo.toml`** ✅ (Done)
   - Update all dependency versions
   - Add new crates (`iroh-tickets`, `n0-future`, `n0-snafu`)
   - Remove `quic-rpc`

2. **Redesign `Iroh` struct in `node.rs`**
   - Remove RPC client fields
   - Store `Endpoint` and protocol handlers directly
   - Implement new initialization pattern

3. **Update `endpoint.rs`**
   - Rename `node_id()` → `id()`
   - Handle `NodeAddr` → `EndpointAddr` rename
   - Update `Connection` methods for infallible returns

### Phase 2: Core Protocols (Estimated: 3-4 days)

4. **Rewrite `blob.rs`**
   - Replace RPC client with `iroh_blobs::api::Store`
   - Update all method signatures
   - Adapt progress/streaming patterns

5. **Rewrite `doc.rs`**
   - Replace RPC client with new docs API
   - Update event subscription patterns

6. **Update `tag.rs`**
   - Fix import paths

### Phase 3: Supporting Code (Estimated: 1-2 days)

7. **Rewrite `net.rs`**
   - Likely merge into `node.rs` or simplify significantly
   - Methods now call `Endpoint` directly

8. **Update `ticket.rs`**
   - Use `iroh-tickets` crate
   - Update type names

9. **Update remaining files**
   - `key.rs` - Minor fixes
   - `author.rs` - Docs API changes
   - `gossip.rs` - API changes
   - `error.rs` - Consider `n0-error`

### Phase 4: Testing & Validation (Estimated: 2-3 days)

10. **Fix compilation errors iteratively**
11. **Run existing tests**
12. **Manual testing of Swift bindings**
13. **Update Swift package if needed**

### Total Estimated Effort: 8-12 days

---

## Risk Assessment

### High Risk
- **Architectural mismatch**: The FFI was designed around RPC; new API is direct
- **API instability**: iroh-blobs 0.97 is marked as "not production quality"
- **Missing documentation**: New APIs may lack comprehensive docs

### Medium Risk
- **Feature parity**: Some old functionality may not have direct equivalents
- **Performance changes**: Direct API may behave differently than RPC
- **Swift binding compatibility**: Generated bindings may need adjustments

### Low Risk
- **Type renames**: Straightforward find/replace operations
- **Import path changes**: Mechanical fixes

### Recommendations

1. **Wait for iroh-blobs stabilization** if production use is planned
2. **Start with minimal FFI** - core blob/networking only
3. **Add features incrementally** after core works
4. **Consider feature flags** to disable unstable parts
5. **Track upstream issues** for API stability announcements

---

## References

- [iroh-blobs 0.90 Changes](https://www.iroh.computer/blog/iroh-blobs-0-90-changes)
- [iroh-blobs 0.90 New Features](https://www.iroh.computer/blog/iroh-blobs-0-90-new-features)
- [iroh 0.91.0 Release](https://www.iroh.computer/blog/iroh-0-91-0-the-last-relay-break)
- [iroh 0.92.0 Release](https://www.iroh.computer/blog/iroh-0-92-0-mdns-improvements)
- [iroh 0.93.0 Release](https://www.iroh.computer/blog/iroh-0-93-iroh-online)
- [iroh 0.94.0 Release](https://www.iroh.computer/blog/iroh-0-94-0-the-endpoint-takeover)
- [iroh 0.95.0 Release](https://www.iroh.computer/blog/iroh-0-95-0-new-relay)
- [iroh-blobs on crates.io](https://crates.io/crates/iroh-blobs)
- [iroh-docs on crates.io](https://crates.io/crates/iroh-docs)
