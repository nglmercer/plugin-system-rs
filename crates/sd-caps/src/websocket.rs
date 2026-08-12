//! `websocket` capability: outbound WebSocket client, backed by `tungstenite`.
//!
//! Deliberately generic. The alternative was an `obs-transport` capability
//! wrapping `obws` host-side, which would have been less code here — but it
//! would also have moved the OBS *protocol* into the host, leaving the plugin
//! as a shell. A plain socket keeps the interesting part (the handshake,
//! request correlation, the scene and stats model) guest-side where it
//! belongs, and gives any future plugin a transport rather than one vendor's
//! client.
//!
//! # Blocking, by design
//!
//! Guest calls are synchronous, so this is a blocking client rather than an
//! async one. Reads use a socket timeout and report "nothing yet" instead of
//! parking, which lets a guest poll without a runtime on either side.
//!
//! # Concurrency
//!
//! A `tungstenite::WebSocket` is one object for both directions and cannot be
//! split without an async stack, so each connection is behind its own mutex.
//! That is not a real constraint here: a plugin's calls are already serialised
//! by its `Store`.

use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use plugin_system::WebSocketProvider;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

/// How long `connect` waits for the TCP handshake.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Ceiling on a single `receive` call, so a guest cannot ask the host to block
/// for longer than its own epoch deadline would allow anyway.
const MAX_RECEIVE_TIMEOUT: Duration = Duration::from_secs(30);

struct Connection {
    socket: Socket,
    /// Cleared when the peer closes or the socket errors, so `is-connected`
    /// can answer without probing.
    open: bool,
}

/// Outbound WebSocket connections, addressed by handle.
pub struct TungsteniteProvider {
    connections: Mutex<HashMap<u32, Connection>>,
    /// Never reused, so a stale handle from a closed connection fails cleanly
    /// rather than addressing somebody else's socket.
    next_handle: AtomicU32,
}

impl TungsteniteProvider {
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
            next_handle: AtomicU32::new(1),
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<u32, Connection>> {
        self.connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Set the read timeout on whichever stream is underneath the TLS layer.
    fn set_read_timeout(socket: &mut Socket, timeout: Option<Duration>) -> Result<(), String> {
        let stream = match socket.get_mut() {
            MaybeTlsStream::Plain(s) => s,
            #[cfg(feature = "websocket")]
            MaybeTlsStream::Rustls(t) => t.get_mut(),
            // `MaybeTlsStream` is non-exhaustive; an unknown variant means we
            // cannot set a timeout, which would turn `receive` into a blocking
            // call. Refuse rather than hang.
            _ => return Err("unsupported stream type: cannot set a read timeout".into()),
        };
        stream
            .set_read_timeout(timeout)
            .map_err(|e| format!("failed to set read timeout: {e}"))
    }

    /// Whether an error just means "nothing arrived in time".
    fn is_timeout(err: &tungstenite::Error) -> bool {
        match err {
            tungstenite::Error::Io(e) => matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ),
            _ => false,
        }
    }
}

impl Default for TungsteniteProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSocketProvider for TungsteniteProvider {
    fn connect(&self, url: &str) -> Result<u32, String> {
        // Parsed up front so a typo is reported as a bad URL rather than as a
        // connection failure.
        let parsed = url::Url::parse(url).map_err(|e| format!("invalid websocket url: {e}"))?;
        match parsed.scheme() {
            "ws" | "wss" => {}
            other => return Err(format!("unsupported scheme '{other}'; expected ws or wss")),
        }

        let (mut socket, _response) =
            tungstenite::client::connect_with_config(parsed.as_str(), None, 3).map_err(|e| {
                // The handshake failure text is what a user needs to see, so
                // keep it rather than flattening to "connection failed".
                format!("websocket connect failed: {e}")
            })?;

        // Default to a bounded read so a guest that forgets a timeout still
        // cannot wedge the host.
        Self::set_read_timeout(&mut socket, Some(CONNECT_TIMEOUT))?;

        let handle = self.next_handle.fetch_add(1, Ordering::SeqCst);
        self.lock()
            .insert(handle, Connection { socket, open: true });

        log::debug!("websocket {handle} connected to {url}");
        Ok(handle)
    }

    fn send(&self, handle: u32, message: &str) -> Result<(), String> {
        let mut conns = self.lock();
        let conn = conns
            .get_mut(&handle)
            .ok_or_else(|| format!("no such websocket handle: {handle}"))?;

        if !conn.open {
            return Err(format!("websocket {handle} is closed"));
        }

        conn.socket
            .send(Message::Text(message.into()))
            .map_err(|e| {
                conn.open = false;
                format!("websocket send failed: {e}")
            })
    }

    fn receive(&self, handle: u32, timeout_ms: u32) -> Result<Option<String>, String> {
        let timeout = Duration::from_millis(timeout_ms as u64).min(MAX_RECEIVE_TIMEOUT);

        let mut conns = self.lock();
        let conn = conns
            .get_mut(&handle)
            .ok_or_else(|| format!("no such websocket handle: {handle}"))?;

        if !conn.open {
            return Err(format!("websocket {handle} is closed"));
        }

        Self::set_read_timeout(&mut conn.socket, Some(timeout))?;

        loop {
            match conn.socket.read() {
                Ok(Message::Text(text)) => return Ok(Some(text.to_string())),
                // Binary frames are not part of any protocol a plugin speaks
                // here; skip rather than fail the read.
                Ok(Message::Binary(_)) => continue,
                // Ping/pong are answered by tungstenite on the next write;
                // they are not something the guest should see.
                Ok(Message::Ping(_)) | Ok(Message::Pong(_)) | Ok(Message::Frame(_)) => continue,
                Ok(Message::Close(_)) => {
                    conn.open = false;
                    return Ok(None);
                }
                Err(e) if Self::is_timeout(&e) => return Ok(None),
                Err(e) => {
                    conn.open = false;
                    return Err(format!("websocket read failed: {e}"));
                }
            }
        }
    }

    fn is_connected(&self, handle: u32) -> bool {
        self.lock().get(&handle).map(|c| c.open).unwrap_or(false)
    }

    fn close(&self, handle: u32) -> Result<(), String> {
        let mut conns = self.lock();
        match conns.remove(&handle) {
            Some(mut conn) => {
                // Best effort: the peer may already be gone, and the socket is
                // being dropped either way.
                let _ = conn.socket.close(None);
                Ok(())
            }
            None => Err(format!("no such websocket handle: {handle}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_malformed_url_before_dialling() {
        let p = TungsteniteProvider::new();
        let err = p.connect("not a url").unwrap_err();
        assert!(err.contains("invalid websocket url"), "got: {err}");
    }

    #[test]
    fn rejects_a_non_websocket_scheme() {
        let p = TungsteniteProvider::new();
        let err = p.connect("http://example.com").unwrap_err();
        assert!(err.contains("unsupported scheme"), "got: {err}");
    }

    /// Operations on a handle that was never issued must fail rather than
    /// silently succeed or panic.
    #[test]
    fn unknown_handles_are_rejected() {
        let p = TungsteniteProvider::new();
        assert!(p.send(42, "hello").is_err());
        assert!(p.receive(42, 10).is_err());
        assert!(p.close(42).is_err());
        assert!(!p.is_connected(42));
    }
}
