//! Server runtime configuration, loaded from `data/config.json`.
//!
//! The file is optional. When it is absent — or `port` is missing/0 — the
//! server binds port `0`, which tells the OS to pick an available port. The
//! real port is printed at startup and reported by `/api/local-ip`, so the QR
//! code and tray "Open in Browser" always point at the right address.
//!
//! Create `data/config.json` with a `port` to keep the same port across
//! restarts. `SD_CORE_BIND_ADDR` (a full socket address) overrides the file.

use serde::Deserialize;

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
}

impl ServerConfig {
    /// The port to hand the OS, or `0` (ephemeral) when none is configured.
    pub fn bind_port(&self) -> u16 {
        self.port.filter(|&p| p > 0).unwrap_or(DEFAULT_PORT)
    }
}

/// Load the optional server config, ignoring a missing or malformed file.
pub fn load() -> ServerConfig {
    std::fs::read_to_string(CONFIG_FILE)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}
