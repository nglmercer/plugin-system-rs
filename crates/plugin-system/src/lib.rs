pub mod context;
pub mod error;
pub mod loader;
pub mod macros;
pub mod manager;
pub mod platform;
pub mod registry;
pub mod traits;

// Re-export commonly used types
pub use context::PluginContext;
pub use error::{PluginError, Result};
pub use loader::{FileLoader, MultiLoader, PluginLoader};
#[cfg(feature = "url-loader")]
pub use loader::UrlLoader;
pub use manager::PluginManager;
pub use platform::{
    copy_cargo_plugin, copy_plugin, library_extension, library_filename, library_path,
};
pub use registry::{new_shared_registry, PluginRegistry, SharedRegistry};
pub use traits::{Plugin, PluginMetadata};
