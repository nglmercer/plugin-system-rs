//! Native implementations of the host capabilities that WASI component
//! plugins call into.
//!
//! A component has no ambient authority, so everything a plugin needs from the
//! machine — CPU stats, audio devices, the keyboard, a socket — is implemented
//! here and handed to the plugin host as a trait object. This is where the
//! platform code that used to live inside the plugins now lives.
//!
//! That relocation is the honest cost of the WASI migration: the native code
//! did not disappear, it changed owner. What changed is *who* runs it. It is
//! now host code, compiled once with the host, rather than something loaded
//! out of a `.so` at runtime and trusted with the whole process.
//!
//! # Safety
//!
//! Unlike `plugin-system`, which is `#![forbid(unsafe_code)]`, this crate is
//! allowed `unsafe` — COM, CoreAudio and X11 leave no choice. Keeping the two
//! apart is the point: the plugin boundary stays verifiably safe while the
//! platform work is quarantined here.
//!
//! # Availability
//!
//! Every provider is optional and feature-gated, and each reports honestly
//! when a platform cannot do what was asked rather than pretending. A host
//! that registers nothing simply grants nothing.

use plugin_system::HostCapabilities;

#[cfg(feature = "system-info")]
pub mod system_info;

#[cfg(feature = "audio")]
pub mod audio;

#[cfg(feature = "input")]
pub mod input;

#[cfg(feature = "websocket")]
pub mod websocket;

/// Build the capability set this host can serve, from whatever is compiled in.
///
/// Providers that fail to initialise are logged and left out rather than
/// aborting startup: a machine with no sound server should still run the rest
/// of the application.
pub fn default_capabilities() -> HostCapabilities {
    let caps = HostCapabilities::new();

    #[cfg(feature = "system-info")]
    let caps = caps.with_system_info(std::sync::Arc::new(system_info::SysinfoProvider::new()));

    // Registered even when the machine has no working sound server: the
    // provider reports `support.master = false`, which a plugin can act on.
    // Withholding it would be indistinguishable from a missing grant.
    #[cfg(feature = "audio")]
    let caps = caps.with_audio(std::sync::Arc::new(audio::NativeAudioProvider::new()));

    #[cfg(feature = "input")]
    let caps = caps.with_input(std::sync::Arc::new(input::RdevProvider::new()));

    #[cfg(feature = "websocket")]
    let caps = caps.with_websocket(std::sync::Arc::new(websocket::TungsteniteProvider::new()));

    log::info!("host capabilities: {:?}", caps.available());
    caps
}
