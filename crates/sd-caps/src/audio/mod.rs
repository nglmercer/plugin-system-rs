//! `audio` capability: master and per-application volume.
//!
//! This is the largest of the four relocations. The PulseAudio, COM and
//! CoreAudio backends below are the ones that used to be compiled into
//! `plugin-volume-master` and loaded into the host with `dlopen`; they are the
//! same code, now compiled *as* the host.
//!
//! # Why the platform types are kept separate
//!
//! [`VolumeControl`] and its `VolumeState` / `AppVolume` are the shapes the
//! platform backends were written against, and they are deliberately not the
//! same types as the ones in the WIT contract. Converting at this boundary
//! costs one `From` impl and keeps ~1200 lines of delicate per-platform code
//! from having to move in lockstep with the plugin ABI.
//!
//! # Threading
//!
//! The backends take `&mut self`; the capability is shared and takes `&self`.
//! A `Mutex` bridges the two, which also serialises access to backends that
//! are not internally synchronised.

use std::sync::Mutex;

use plugin_system::{AudioProvider, AudioSupport};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
use unsupported as platform;

/// Master volume as the platform backends report it.
#[derive(Debug, Clone, Default)]
pub struct VolumeState {
    pub master_volume: f32,
    pub muted: bool,
    pub default_device_name: String,
}

/// One application's stream, as the platform backends report it.
#[derive(Debug, Clone, Default)]
pub struct AppVolume {
    /// Backend handle addressing this exact stream. Empty means the backend
    /// has no stream-level identity, in which case the name is used instead.
    pub id: String,
    pub name: String,
    /// What the stream is playing, when the backend knows. Empty otherwise.
    pub title: String,
    /// Freedesktop icon name hint. Empty when unknown.
    pub icon: String,
    pub volume: f32,
    pub muted: bool,
    pub pid: Option<u32>,
}

/// What every platform backend implements.
pub trait VolumeControl: Send + Sync {
    fn get_master_volume(&mut self) -> Result<VolumeState, String>;
    fn set_master_volume(&mut self, volume: f32) -> Result<(), String>;
    fn set_muted(&mut self, muted: bool) -> Result<(), String>;
    fn get_app_volumes(&mut self) -> Result<Vec<AppVolume>, String>;
    fn set_app_volume(&mut self, app_name: &str, volume: f32) -> Result<(), String>;
    fn set_app_muted(&mut self, app_name: &str, muted: bool) -> Result<(), String>;
}

/// The `audio` capability, backed by whichever platform backend was compiled.
pub struct NativeAudioProvider {
    controller: Mutex<Box<dyn VolumeControl>>,
    support: AudioSupport,
}

impl NativeAudioProvider {
    /// Connect to the platform's audio system.
    ///
    /// Infallible: every backend supplies a fallback that reports errors
    /// rather than failing to construct. Support is then *probed* rather than
    /// assumed — a Linux box with no PulseAudio socket compiles the same code
    /// as one with a working sound server, and only a real call can tell them
    /// apart. Reporting `master: false` lets a plugin grey out its control
    /// instead of showing an error on every poll.
    pub fn new() -> Self {
        let mut controller = platform::create_controller();

        let master = match controller.get_master_volume() {
            Ok(_) => true,
            Err(e) => {
                log::warn!("audio: master volume unavailable on this host: {e}");
                false
            }
        };

        Self {
            controller: Mutex::new(controller),
            support: AudioSupport {
                master,
                // Per-app control needs both a platform that offers it and a
                // working connection to use it through.
                per_app: master && platform::per_app_supported(),
            },
        }
    }

    fn with<T>(
        &self,
        f: impl FnOnce(&mut dyn VolumeControl) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self
            .controller
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(guard.as_mut())
    }
}

/// `new` is infallible and takes no arguments, so `Default` is simply the
/// right name for it — and clippy's `new_without_default` is right to say so.
/// It probes the platform like `new` does, since a provider that has not
/// probed would report support it cannot deliver.
impl Default for NativeAudioProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioProvider for NativeAudioProvider {
    fn support(&self) -> AudioSupport {
        self.support
    }

    fn get_master(&self) -> Result<plugin_system::VolumeState, String> {
        self.with(|c| c.get_master_volume()).map(|s| plugin_system::VolumeState {
            volume: s.master_volume,
            muted: s.muted,
            device_name: s.default_device_name,
        })
    }

    fn set_master(&self, volume: f32) -> Result<(), String> {
        self.with(|c| c.set_master_volume(volume))
    }

    fn set_master_mute(&self, muted: bool) -> Result<(), String> {
        self.with(|c| c.set_muted(muted))
    }

    fn list_apps(&self) -> Result<Vec<plugin_system::AppVolume>, String> {
        self.with(|c| c.get_app_volumes()).map(|apps| {
            apps.into_iter()
                .map(|a| plugin_system::AppVolume {
                    // Backends that can identify a single stream supply an id;
                    // the rest fall back to the application name, which is
                    // ambiguous when an app owns several streams but is the
                    // best those platforms can offer.
                    id: if a.id.is_empty() { a.name.clone() } else { a.id },
                    name: a.name,
                    title: a.title,
                    icon: a.icon,
                    volume: a.volume,
                    muted: a.muted,
                    pid: a.pid,
                })
                .collect()
        })
    }

    fn set_app_volume(&self, id: &str, volume: f32) -> Result<(), String> {
        self.with(|c| c.set_app_volume(id, volume))
    }

    fn set_app_mute(&self, id: &str, muted: bool) -> Result<(), String> {
        self.with(|c| c.set_app_muted(id, muted))
    }
}
