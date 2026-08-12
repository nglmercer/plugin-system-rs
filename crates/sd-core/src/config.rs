//! Server runtime configuration, loaded from `data/config.json`.
//!
//! The file is optional. When it is absent — or `port` is missing/0 — the
//! server binds port `0`, which tells the OS to pick an available port. The
//! real port is printed at startup and reported by `/api/local-ip`, so the QR
//! code and tray "Open in Browser" always point at the right address.
//!
//! Create `data/config.json` with a `port` to keep the same port across
//! restarts. `SD_CORE_BIND_ADDR` (a full socket address) overrides the file.
//!
//! The bind *address* defaults to loopback. This daemon injects keystrokes,
//! drives OBS and loads uploaded plugins, so reaching it from the LAN is an
//! explicit decision — set `host` in the config file (or `SD_CORE_BIND_ADDR`)
//! to opt in, and know that the API token is then the only thing between the
//! network and the desktop.

use serde::Deserialize;
use std::net::IpAddr;

/// Address to bind when nothing configures one.
pub const DEFAULT_HOST: IpAddr = IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);

/// Port to bind when no configuration sets one. `0` means "any available
/// port": the server never fails on a busy port.
pub const DEFAULT_PORT: u16 = 0;

/// Config file path, relative to the CWD — the same convention the dashboard
/// config uses (`data/dashboard.json`).
pub const CONFIG_FILE: &str = "data/config.json";

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerConfig {
    /// Fixed HTTP port to bind. `None` or `0` means bind an ephemeral port.
    #[serde(default)]
    pub port: Option<u16>,
    /// Address to bind. Omit for loopback; use `"0.0.0.0"` to accept
    /// connections from the network.
    #[serde(default)]
    pub host: Option<String>,
}

impl ServerConfig {
    /// The port to hand the OS, or `0` (ephemeral) when none is configured.
    pub fn bind_port(&self) -> u16 {
        self.port.filter(|&p| p > 0).unwrap_or(DEFAULT_PORT)
    }

    /// The address to bind, defaulting to loopback.
    ///
    /// An unparseable `host` falls back to loopback rather than failing the
    /// start: the safe direction for a typo is "less reachable", never more.
    pub fn bind_host(&self) -> IpAddr {
        match self.host.as_deref().map(str::trim) {
            None | Some("") => DEFAULT_HOST,
            Some(raw) => raw.parse().unwrap_or_else(|_| {
                tracing::warn!(
                    host = raw,
                    "config `host` is not a valid IP address; binding loopback instead"
                );
                DEFAULT_HOST
            }),
        }
    }
}

/// Load the optional server config, ignoring a missing or malformed file.
pub fn load() -> ServerConfig {
    std::fs::read_to_string(CONFIG_FILE)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default must stay loopback. Binding every interface is what put an
    /// unauthenticated keystroke-injection API on the LAN.
    #[test]
    fn host_defaults_to_loopback() {
        assert_eq!(ServerConfig::default().bind_host(), DEFAULT_HOST);
        assert!(ServerConfig::default().bind_host().is_loopback());
    }

    #[test]
    fn an_explicit_host_is_honoured() {
        let cfg = ServerConfig {
            port: None,
            host: Some("0.0.0.0".to_string()),
        };
        assert_eq!(cfg.bind_host(), IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
    }

    /// A typo must fail closed.
    #[test]
    fn an_unparseable_host_falls_back_to_loopback() {
        let cfg = ServerConfig {
            port: None,
            host: Some("not-an-ip".to_string()),
        };
        assert!(cfg.bind_host().is_loopback());
    }
}
