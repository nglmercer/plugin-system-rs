pub mod context;
pub mod error;
pub mod loader;
pub mod macros;
pub mod manager;
pub mod platform;
pub mod plugin_info;
pub mod registry;
pub mod traits;

pub use context::PluginContext;
pub use error::{PluginError, Result};
#[cfg(feature = "url-loader")]
pub use loader::UrlLoader;
pub use loader::{FileLoader, MultiLoader, PluginLoader};
pub use manager::PluginManager;
pub use platform::{
    copy_cargo_plugin, copy_plugin, library_extension, library_filename, library_path,
};
pub use plugin_info::{PluginInfo, PluginResult};
pub use registry::{new_shared_registry, PluginRegistry, SharedRegistry};
pub use traits::{Plugin, PluginMetadata};

pub use plugin_macros::{define_plugin, plugin_export, plugin_interface};
