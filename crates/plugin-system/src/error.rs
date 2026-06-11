use std::path::PathBuf;

use semver::Error as SemverError;

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Failed to load library '{path}': {reason}")]
    LibraryLoad { path: PathBuf, reason: String },

    #[error("Symbol '{symbol}' not found in plugin library")]
    SymbolNotFound { symbol: String },

    #[error("Plugin '{name}' failed to load: {reason}")]
    PluginLoad { name: String, reason: String },

    #[error("Plugin '{name}' not found in registry")]
    PluginNotFound { name: String },

    #[error("Failed to unload plugin '{name}': {reason}")]
    UnloadFailed { name: String, reason: String },

    #[error("Dependency '{dependency}' required by '{plugin}' is not loaded")]
    MissingDependency { plugin: String, dependency: String },

    #[error("Version incompatibility: plugin '{name}' requires '{required}', found '{found}'")]
    VersionIncompatible {
        name: String,
        required: String,
        found: String,
    },

    #[error("Plugin '{name}' declared an invalid semver requirement: {reason}")]
    InvalidSemverRequirement { name: String, reason: String },

    #[error("Plugin '{name}' panicked during on_load: {reason}")]
    OnLoadPanic { name: String, reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to download plugin from '{url}': {reason}")]
    DownloadFailed { url: String, reason: String },

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Plugin cache error: {0}")]
    CacheError(String),

    #[error("HTTP error: {status} {url}")]
    HttpError { url: String, status: u16 },
}

pub type Result<T> = std::result::Result<T, PluginError>;

impl PluginError {
    pub fn from_semver(name: &str, err: &SemverError) -> Self {
        PluginError::InvalidSemverRequirement {
            name: name.to_string(),
            reason: err.to_string(),
        }
    }

    pub fn from_lock_error(lock_name: &str, reason: &str) -> Self {
        PluginError::UnloadFailed {
            name: lock_name.to_string(),
            reason: format!("lock poisoned: {}", reason),
        }
    }
}
