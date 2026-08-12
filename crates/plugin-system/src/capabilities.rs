//! Host capability providers.
//!
//! A component has no ambient authority: no filesystem, no network, no way to
//! reach an audio device or a keyboard. Anything a plugin needs from the
//! outside world arrives through one of these traits, which the embedding
//! application implements and hands to the [`crate::PluginManager`].
//!
//! # Why traits rather than direct implementations
//!
//! The native code these replace is exactly the code that cannot be written
//! without `unsafe` — COM, CoreAudio, X11, uinput. Keeping the implementations
//! behind traits lets `plugin-system` stay `#![forbid(unsafe_code)]` while the
//! platform work lives in a crate that is allowed to do what it must.
//!
//! It also means a host can decline. Every provider is optional: a host that
//! registers no [`AudioProvider`] simply reports audio as unsupported, and a
//! test can substitute a fake without a sound card in sight.
//!
//! # Granting
//!
//! Registering a provider makes a capability *available*; it does not make it
//! *reachable*. A plugin must also list the capability in its manifest, and
//! the host checks that on every call. See [`crate::manifest::PluginManifest`].

use std::sync::Arc;

/// A snapshot of the machine, mirroring the `system-info` WIT record.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemStats {
    pub cpu_usage: f64,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub memory_total: u64,
    pub memory_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    /// 1, 5 and 15 minute load averages; zeroed where the platform has none.
    pub load_avg: [f64; 3],
    pub uptime_seconds: u64,
    pub process_count: u32,
    pub thread_count: u32,
}

/// Read-only view of the machine.
pub trait SystemInfoProvider: Send + Sync {
    fn get_stats(&self) -> std::result::Result<SystemStats, String>;
}

/// Master volume and the device it belongs to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VolumeState {
    pub volume: f32,
    pub muted: bool,
    pub device_name: String,
}

/// One application's audio stream.
#[derive(Debug, Clone, PartialEq)]
pub struct AppVolume {
    pub id: String,
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub pid: Option<u32>,
}

/// What this host's audio backend can actually do.
///
/// Reported rather than discovered by trial: macOS has no public per-app
/// volume API, so a plugin should be able to grey out that control instead of
/// showing an error on every call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioSupport {
    pub master: bool,
    pub per_app: bool,
}

/// Master and per-application volume control.
///
/// Takes `&self` throughout: providers are shared behind an `Arc` and called
/// from whichever plugin holds the grant, so any mutable backend state is the
/// implementation's problem to lock.
pub trait AudioProvider: Send + Sync {
    fn support(&self) -> AudioSupport;
    fn get_master(&self) -> std::result::Result<VolumeState, String>;
    fn set_master(&self, volume: f32) -> std::result::Result<(), String>;
    fn set_master_mute(&self, muted: bool) -> std::result::Result<(), String>;
    fn list_apps(&self) -> std::result::Result<Vec<AppVolume>, String>;
    fn set_app_volume(&self, id: &str, volume: f32) -> std::result::Result<(), String>;
    fn set_app_mute(&self, id: &str, muted: bool) -> std::result::Result<(), String>;
}

/// Synthetic keyboard input, and reading a chord back.
///
/// The riskiest capability in this module: granting it hands over both the
/// keyboard and a global view of what is typed on it.
pub trait InputProvider: Send + Sync {
    fn send_key(&self, key: &str) -> std::result::Result<(), String>;
    fn send_hotkey(&self, modifiers: &[String], key: &str) -> std::result::Result<(), String>;
    fn send_text(&self, text: &str) -> std::result::Result<(), String>;

    /// Block until a chord is pressed, or `timeout_ms` elapses.
    fn record_hotkey(&self, timeout_ms: u32) -> std::result::Result<String, String>;

    /// Abandon an in-progress recording.
    fn reset_recording(&self);
}

/// Outbound WebSocket client, addressed by opaque handle.
pub trait WebSocketProvider: Send + Sync {
    fn connect(&self, url: &str) -> std::result::Result<u32, String>;
    fn send(&self, handle: u32, message: &str) -> std::result::Result<(), String>;
    /// Returns `Ok(None)` when `timeout_ms` elapsed with nothing queued.
    fn receive(
        &self,
        handle: u32,
        timeout_ms: u32,
    ) -> std::result::Result<Option<String>, String>;
    fn is_connected(&self, handle: u32) -> bool;
    fn close(&self, handle: u32) -> std::result::Result<(), String>;
}

/// The set of capability implementations a host offers.
///
/// Cloning is cheap — every field is an `Arc` — so each plugin instance can
/// hold its own copy without duplicating the backends.
#[derive(Clone, Default)]
pub struct HostCapabilities {
    pub system_info: Option<Arc<dyn SystemInfoProvider>>,
    pub audio: Option<Arc<dyn AudioProvider>>,
    pub input: Option<Arc<dyn InputProvider>>,
    pub websocket: Option<Arc<dyn WebSocketProvider>>,
}

impl HostCapabilities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_system_info(mut self, provider: Arc<dyn SystemInfoProvider>) -> Self {
        self.system_info = Some(provider);
        self
    }

    pub fn with_audio(mut self, provider: Arc<dyn AudioProvider>) -> Self {
        self.audio = Some(provider);
        self
    }

    pub fn with_input(mut self, provider: Arc<dyn InputProvider>) -> Self {
        self.input = Some(provider);
        self
    }

    pub fn with_websocket(mut self, provider: Arc<dyn WebSocketProvider>) -> Self {
        self.websocket = Some(provider);
        self
    }

    /// Capability names this host can actually serve.
    pub fn available(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if self.system_info.is_some() {
            names.push(SYSTEM_INFO);
        }
        if self.audio.is_some() {
            names.push(AUDIO);
        }
        if self.input.is_some() {
            names.push(INPUT);
        }
        if self.websocket.is_some() {
            names.push(WEBSOCKET);
        }
        names
    }
}

impl std::fmt::Debug for HostCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostCapabilities")
            .field("available", &self.available())
            .finish()
    }
}

/// Capability names, as they appear in a manifest's `capabilities` list.
pub const SYSTEM_INFO: &str = "system-info";
pub const AUDIO: &str = "audio";
pub const INPUT: &str = "input";
pub const WEBSOCKET: &str = "websocket";

/// Every capability the host knows how to name.
///
/// Used to reject a manifest that asks for something misspelled, rather than
/// silently never granting it.
pub const ALL: &[&str] = &[SYSTEM_INFO, AUDIO, INPUT, WEBSOCKET];

/// Whether `name` is a capability this crate defines.
pub fn is_known(name: &str) -> bool {
    ALL.contains(&name)
}
