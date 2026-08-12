//! Plugin framework for WebAssembly component plugins running on WASI
//! Preview 2.
//!
//! # No unsafe code
//!
//! This crate contains no `unsafe`, and `forbid(unsafe_code)` makes that a
//! compile error to violate rather than a claim in a comment.
//!
//! That is a direct consequence of having exactly one plugin backend. The
//! shared-library ABIs this crate used to support — the Rust trait object and
//! the flat C ABI — both went through `dlopen`, and opening a library,
//! resolving symbols, and calling through raw function pointers cannot be
//! checked by the compiler. A plugin built against a different rustc or a
//! different dependency set produced a vtable the host would happily call and
//! then crash on, and a misbehaving plugin took the whole process with it.
//!
//! WebAssembly needs none of that: the boundary is a versioned WIT contract
//! and the sandbox is enforced by wasmtime at runtime rather than by the
//! host's own pointer discipline. A plugin that loops forever hits an epoch
//! deadline, one that allocates without bound hits its memory ceiling, and one
//! that panics traps at the boundary — in every case the host returns an error
//! and stays up.
#![forbid(unsafe_code)]

pub mod capabilities;
pub mod context;
pub mod error;
pub mod handler;
pub mod loader;
pub mod macros;
pub mod manager;
pub mod manifest;
pub mod naming;
pub mod plugin_info;
pub mod registry;
pub mod traits;
pub mod wasm;

pub use capabilities::{
    AppVolume, AudioProvider, AudioSupport, HostCapabilities, InputProvider, SystemInfoProvider,
    SystemStats, VolumeState, WebSocketProvider,
};
pub use context::PluginContext;
pub use error::{PluginError, Result};
pub use handler::{
    new_shared_command_registry, CommandHandler, CommandRegistry, PluginCommandHandler,
    SharedCommandRegistry,
};
#[cfg(feature = "url-loader")]
pub use loader::UrlLoader;
pub use loader::{FileLoader, MultiLoader, PluginLoader};
pub use manager::{PluginManager, PLUGIN_EXTENSION};
pub use naming::{canonical_plugin_name, canonical_plugin_name_from_path, plugin_file_stem};
pub use plugin_info::{PluginInfo, PluginResult};
pub use registry::{new_shared_registry, PluginRegistry, SharedRegistry};
pub use serde_json;
pub use traits::{command_to_json, CommandResult, Plugin, PluginDependency, PluginMetadata};

pub use manifest::{load_plugin_manifest, Abi, Manifest, PluginManifest, ResourceLimits};
