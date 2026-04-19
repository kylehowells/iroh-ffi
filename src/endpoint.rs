use std::{sync::Arc, time::Duration};

use iroh::endpoint;
use iroh_base::TransportAddr;
use tokio::sync::Mutex;

use crate::{IrohError, NodeAddr};

#[derive(Clone, uniffi::Object)]
pub struct Endpoint(endpoint::Endpoint);

impl Endpoint {
    pub fn new(ep: endpoint::Endpoint) -> Self {
        Endpoint(ep)
    }
}

#[uniffi::export]
impl Endpoint {
    #[uniffi::method]
    /// The string representation of this endpoint's EndpointId.
    pub fn node_id(&self) -> Result<String, IrohError> {
        let id = self.0.id();
        Ok(id.to_string())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn connect(
        &self,
        node_addr: &NodeAddr,
        alpn: &[u8],
    ) -> Result<Connection, IrohError> {
        let endpoint_addr: iroh::EndpointAddr = node_addr.clone().try_into()?;
        let conn = self.0.connect(endpoint_addr, alpn).await.map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Connection(conn))
    }
}

#[derive(uniffi::Object)]
pub struct Connecting(Mutex<Option<endpoint::Connecting>>);

impl Connecting {
    pub fn new(conn: endpoint::Connecting) -> Self {
        Connecting(Mutex::new(Some(conn)))
    }
}

#[uniffi::export]
impl Connecting {
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn connect(&self) -> Result<Connection, IrohError> {
        match self.0.lock().await.take() {
            Some(conn) => {
                let conn = conn.await.map_err(anyhow::Error::from)?;
                Ok(Connection(conn))
            }
            None => Err(anyhow::anyhow!("already used").into()),
        }
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn alpn(&self) -> Result<Vec<u8>, IrohError> {
        match &mut *self.0.lock().await {
            Some(conn) => {
                let alpn = conn.alpn().await.map_err(|e| anyhow::anyhow!("{e}"))?;
                Ok(alpn)
            }
            None => Err(anyhow::anyhow!("already used").into()),
        }
    }
}

#[derive(uniffi::Object)]
pub struct Connection(endpoint::Connection);

impl From<endpoint::Connection> for Connection {
    fn from(value: endpoint::Connection) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ConnectionPathKind {
    /// No selected transport path is available yet.
    Unknown,
    /// The current selected transport path is a direct IP connection.
    Direct,
    /// The current selected transport path is a relay connection.
    Relay,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConnectionPathState {
    /// The currently selected transport-path kind.
    pub kind: ConnectionPathKind,
    /// The remote direct IP transport address, if the selected path is direct.
    pub direct_address: Option<String>,
    /// The remote relay URL, if the selected path is relay-backed.
    pub relay_url: Option<String>,
    /// The selected path RTT in milliseconds, if a selected path is available.
    pub rtt_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConnectionPathInfo {
    /// The transport-path kind for this candidate path.
    pub kind: ConnectionPathKind,
    /// The remote direct IP transport address, if this path is direct.
    pub direct_address: Option<String>,
    /// The remote relay URL, if this path is relay-backed.
    pub relay_url: Option<String>,
    /// The path RTT in milliseconds, if available.
    pub rtt_ms: Option<u64>,
    /// Whether this path is currently selected.
    pub is_selected: bool,
}

impl ConnectionPathState {
    fn unknown() -> Self {
        Self {
            kind: ConnectionPathKind::Unknown,
            direct_address: None,
            relay_url: None,
            rtt_ms: None,
        }
    }
}

fn connection_path_info_for_transport(
    transport: Option<&TransportAddr>,
    rtt: Option<Duration>,
    is_selected: bool,
) -> ConnectionPathInfo {
    let rtt_ms = rtt.map(|value| value.as_millis() as u64);

    match transport {
        Some(TransportAddr::Ip(addr)) => ConnectionPathInfo {
            kind: ConnectionPathKind::Direct,
            direct_address: Some(addr.to_string()),
            relay_url: None,
            rtt_ms,
            is_selected,
        },
        Some(TransportAddr::Relay(url)) => ConnectionPathInfo {
            kind: ConnectionPathKind::Relay,
            direct_address: None,
            relay_url: Some(url.to_string()),
            rtt_ms,
            is_selected,
        },
        Some(_) | None => ConnectionPathInfo {
            kind: ConnectionPathKind::Unknown,
            direct_address: None,
            relay_url: None,
            rtt_ms,
            is_selected,
        },
    }
}

fn connection_path_state_for_selected_transport(
    selected_transport: Option<&TransportAddr>,
    rtt: Option<Duration>,
) -> ConnectionPathState {
    let info = connection_path_info_for_transport(selected_transport, rtt, true);
    ConnectionPathState {
        kind: info.kind,
        direct_address: info.direct_address,
        relay_url: info.relay_url,
        rtt_ms: info.rtt_ms,
    }
}

#[uniffi::export]
impl Connection {
    #[uniffi::method]
    pub fn alpn(&self) -> Vec<u8> {
        self.0.alpn().to_vec()
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn open_uni(&self) -> Result<SendStream, IrohError> {
        let s = self.0.open_uni().await.map_err(anyhow::Error::from)?;
        Ok(SendStream::new(s))
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn accept_uni(&self) -> Result<RecvStream, IrohError> {
        let r = self.0.accept_uni().await.map_err(anyhow::Error::from)?;
        Ok(RecvStream::new(r))
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn open_bi(&self) -> Result<BiStream, IrohError> {
        let (s, r) = self.0.open_bi().await.map_err(anyhow::Error::from)?;
        Ok(BiStream {
            send: SendStream::new(s),
            recv: RecvStream::new(r),
        })
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn accept_bi(&self) -> Result<BiStream, IrohError> {
        let (s, r) = self.0.accept_bi().await.map_err(anyhow::Error::from)?;
        Ok(BiStream {
            send: SendStream::new(s),
            recv: RecvStream::new(r),
        })
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn read_datagram(&self) -> Result<Vec<u8>, IrohError> {
        let res = self.0.read_datagram().await.map_err(anyhow::Error::from)?;
        Ok(res.to_vec())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn closed(&self) -> String {
        let err = self.0.closed().await;
        err.to_string()
    }

    #[uniffi::method]
    pub fn close_reason(&self) -> Option<String> {
        let err = self.0.close_reason();
        err.map(|s| s.to_string())
    }

    #[uniffi::method]
    pub fn close(&self, error_code: u64, reason: &[u8]) -> Result<(), IrohError> {
        let code = endpoint::VarInt::from_u64(error_code).map_err(anyhow::Error::from)?;
        self.0.close(code, reason);
        Ok(())
    }

    #[uniffi::method]
    pub fn send_datagram(&self, data: Vec<u8>) -> Result<(), IrohError> {
        self.0
            .send_datagram(data.into())
            .map_err(anyhow::Error::from)?;
        Ok(())
    }

    #[uniffi::method]
    pub fn max_datagram_size(&self) -> Option<u64> {
        self.0.max_datagram_size().map(|s| s as _)
    }

    #[uniffi::method]
    pub fn datagram_send_buffer_space(&self) -> u64 {
        self.0.datagram_send_buffer_space() as _
    }

    #[uniffi::method]
    pub fn remote_node_id(&self) -> String {
        let id = self.0.remote_id();
        id.to_string()
    }

    /// Get the round-trip time to the peer in milliseconds.
    /// Returns 0 if no selected path is available yet.
    #[uniffi::method]
    pub fn rtt(&self) -> u64 {
        // Use paths() watcher to find the selected path's RTT.
        use iroh::Watcher;
        let paths = self.0.paths().get();
        let mut rtt_ms = 0u64;
        for path in paths.iter() {
            if path.is_selected() {
                if let Some(rtt) = path.rtt() {
                    rtt_ms = rtt.as_millis() as u64;
                }
                break;
            }
        }
        rtt_ms
    }

    /// Get the currently selected transport path for this connection.
    ///
    /// This reports whether the active transmission path is relay-backed or direct IP,
    /// along with the selected path RTT when available.
    #[uniffi::method]
    pub fn current_path_state(&self) -> ConnectionPathState {
        use iroh::Watcher;
        let paths = self.0.paths().get();

        for path in paths.iter() {
            if path.is_selected() {
                return connection_path_state_for_selected_transport(
                    Some(path.remote_addr()),
                    path.rtt(),
                );
            }
        }

        ConnectionPathState::unknown()
    }

    /// Returns all known transport paths for this connection.
    #[uniffi::method]
    pub fn path_infos(&self) -> Vec<ConnectionPathInfo> {
        use iroh::Watcher;
        let paths = self.0.paths().get();
        paths.iter()
            .map(|path| {
                connection_path_info_for_transport(
                    Some(path.remote_addr()),
                    path.rtt(),
                    path.is_selected(),
                )
            })
            .collect()
    }

    #[uniffi::method]
    pub fn stable_id(&self) -> u64 {
        self.0.stable_id() as _
    }

    #[uniffi::method]
    pub fn set_max_concurrent_uni_stream(&self, count: u64) -> Result<(), IrohError> {
        let n = endpoint::VarInt::from_u64(count).map_err(anyhow::Error::from)?;
        self.0.set_max_concurrent_uni_streams(n);
        Ok(())
    }

    #[uniffi::method]
    pub fn set_receive_window(&self, count: u64) -> Result<(), IrohError> {
        let n = endpoint::VarInt::from_u64(count).map_err(anyhow::Error::from)?;
        self.0.set_receive_window(n);
        Ok(())
    }

    #[uniffi::method]
    pub fn set_max_concurrent_bii_stream(&self, count: u64) -> Result<(), IrohError> {
        let n = endpoint::VarInt::from_u64(count).map_err(anyhow::Error::from)?;
        self.0.set_max_concurrent_bi_streams(n);
        Ok(())
    }
}

#[derive(uniffi::Object)]
pub struct BiStream {
    send: SendStream,
    recv: RecvStream,
}

#[uniffi::export]
impl BiStream {
    #[uniffi::method]
    pub fn send(&self) -> SendStream {
        self.send.clone()
    }

    #[uniffi::method]
    pub fn recv(&self) -> RecvStream {
        self.recv.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_selected_direct_transport_to_direct_path_state() {
        let transport: TransportAddr = TransportAddr::Ip(
            "127.0.0.1:7777".parse::<std::net::SocketAddr>().unwrap(),
        );

        let state = connection_path_state_for_selected_transport(
            Some(&transport),
            Some(Duration::from_millis(42)),
        );

        assert_eq!(state.kind, ConnectionPathKind::Direct);
        assert_eq!(state.direct_address.as_deref(), Some("127.0.0.1:7777"));
        assert_eq!(state.relay_url, None);
        assert_eq!(state.rtt_ms, Some(42));
    }

    #[test]
    fn maps_selected_relay_transport_to_relay_path_state() {
        let transport: TransportAddr = TransportAddr::Relay(
            "https://relay.example.test".parse().unwrap(),
        );

        let state = connection_path_state_for_selected_transport(
            Some(&transport),
            Some(Duration::from_millis(15)),
        );

        assert_eq!(state.kind, ConnectionPathKind::Relay);
        assert_eq!(state.direct_address, None);
        assert_eq!(state.relay_url.as_deref(), Some("https://relay.example.test/"));
        assert_eq!(state.rtt_ms, Some(15));
    }

    #[test]
    fn maps_missing_selected_transport_to_unknown_path_state() {
        let state = connection_path_state_for_selected_transport(None, None);

        assert_eq!(state.kind, ConnectionPathKind::Unknown);
        assert_eq!(state.direct_address, None);
        assert_eq!(state.relay_url, None);
        assert_eq!(state.rtt_ms, None);
    }

    #[test]
    fn maps_selected_direct_transport_to_path_info() {
        let transport: TransportAddr = TransportAddr::Ip(
            "127.0.0.1:7777".parse::<std::net::SocketAddr>().unwrap(),
        );

        let info = connection_path_info_for_transport(
            Some(&transport),
            Some(Duration::from_millis(42)),
            true,
        );

        assert_eq!(info.kind, ConnectionPathKind::Direct);
        assert_eq!(info.direct_address.as_deref(), Some("127.0.0.1:7777"));
        assert_eq!(info.relay_url, None);
        assert_eq!(info.rtt_ms, Some(42));
        assert!(info.is_selected);
    }
}

#[derive(Clone, uniffi::Object)]
pub struct SendStream(Arc<Mutex<endpoint::SendStream>>);

impl SendStream {
    fn new(s: endpoint::SendStream) -> Self {
        SendStream(Arc::new(Mutex::new(s)))
    }
}

#[uniffi::export]
impl SendStream {
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn write(&self, buf: &[u8]) -> Result<u64, IrohError> {
        let mut s = self.0.lock().await;
        let written = s.write(buf).await.map_err(anyhow::Error::from)?;
        Ok(written as _)
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn write_all(&self, buf: &[u8]) -> Result<(), IrohError> {
        let mut s = self.0.lock().await;
        s.write_all(buf).await.map_err(anyhow::Error::from)?;
        Ok(())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn finish(&self) -> Result<(), IrohError> {
        let mut s = self.0.lock().await;
        s.finish().map_err(anyhow::Error::from)?;
        Ok(())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn reset(&self, error_code: u64) -> Result<(), IrohError> {
        let error_code = endpoint::VarInt::from_u64(error_code).map_err(anyhow::Error::from)?;
        let mut s = self.0.lock().await;
        s.reset(error_code).map_err(anyhow::Error::from)?;
        Ok(())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn set_priority(&self, p: i32) -> Result<(), IrohError> {
        let s = self.0.lock().await;
        s.set_priority(p).map_err(anyhow::Error::from)?;
        Ok(())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn priority(&self) -> Result<i32, IrohError> {
        let s = self.0.lock().await;
        let p = s.priority().map_err(anyhow::Error::from)?;
        Ok(p)
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn stopped(&self) -> Result<Option<u64>, IrohError> {
        let s = self.0.lock().await;
        let res = s.stopped().await.map_err(anyhow::Error::from)?;
        let res = res.map(|r| r.into_inner());
        Ok(res)
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn id(&self) -> String {
        let r = self.0.lock().await;
        r.id().to_string()
    }
}

#[derive(Clone, uniffi::Object)]
pub struct RecvStream(Arc<Mutex<endpoint::RecvStream>>);

impl RecvStream {
    fn new(s: endpoint::RecvStream) -> Self {
        RecvStream(Arc::new(Mutex::new(s)))
    }
}

#[uniffi::export]
impl RecvStream {
    #[uniffi::method(async_runtime = "tokio")]
    pub async fn read(&self, size_limit: u32) -> Result<Vec<u8>, IrohError> {
        let mut buf = vec![0u8; size_limit as _];
        let mut r = self.0.lock().await;
        let res = r.read(&mut buf).await.map_err(anyhow::Error::from)?;
        let len = res.unwrap_or(0);
        buf.truncate(len);
        Ok(buf)
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn read_exact(&self, size: u32) -> Result<Vec<u8>, IrohError> {
        let mut buf = vec![0u8; size as _];
        let mut r = self.0.lock().await;
        r.read_exact(&mut buf).await.map_err(anyhow::Error::from)?;
        Ok(buf)
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn read_to_end(&self, size_limit: u32) -> Result<Vec<u8>, IrohError> {
        let mut r = self.0.lock().await;
        let res = r
            .read_to_end(size_limit as _)
            .await
            .map_err(anyhow::Error::from)?;
        Ok(res)
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn id(&self) -> String {
        let r = self.0.lock().await;
        r.id().to_string()
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn stop(&self, error_code: u64) -> Result<(), IrohError> {
        let error_code = endpoint::VarInt::from_u64(error_code).map_err(anyhow::Error::from)?;
        let mut r = self.0.lock().await;
        r.stop(error_code).map_err(anyhow::Error::from)?;
        Ok(())
    }

    #[uniffi::method(async_runtime = "tokio")]
    pub async fn received_reset(&self) -> Result<Option<u64>, IrohError> {
        let mut r = self.0.lock().await;
        let code = r.received_reset().await.map_err(anyhow::Error::from)?;
        let code = code.map(|c| c.into_inner());
        Ok(code)
    }
}
