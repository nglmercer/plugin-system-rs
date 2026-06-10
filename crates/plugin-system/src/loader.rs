use std::path::{Path, PathBuf};

use crate::error::{PluginError, Result};

/// Trait for loading plugin binaries from different sources.
pub trait PluginLoader {
    /// Load a plugin binary and return its bytes.
    fn load(&self) -> Result<Vec<u8>>;

    /// Get the source description (file path or URL).
    fn source(&self) -> String;

    /// Check if the source exists and is accessible.
    fn exists(&self) -> bool;
}

/// Loader for plugins from local file paths.
pub struct FileLoader {
    path: PathBuf,
}

impl FileLoader {
    /// Create a new file loader for the given path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Get the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl PluginLoader for FileLoader {
    fn load(&self) -> Result<Vec<u8>> {
        log::info!("Loading plugin from file: {}", self.path.display());
        std::fs::read(&self.path).map_err(PluginError::Io)
    }

    fn source(&self) -> String {
        self.path.display().to_string()
    }

    fn exists(&self) -> bool {
        self.path.exists()
    }
}

/// Loader for plugins from URLs.
#[cfg(feature = "url-loader")]
pub struct UrlLoader {
    url: String,
    cache_dir: Option<PathBuf>,
}

#[cfg(feature = "url-loader")]
impl UrlLoader {
    /// Create a new URL loader.
    ///
    /// # Arguments
    /// * `url` - The URL to download the plugin from
    /// * `cache_dir` - Optional directory to cache downloaded plugins
    pub fn new(url: impl Into<String>, cache_dir: Option<PathBuf>) -> Self {
        Self {
            url: url.into(),
            cache_dir,
        }
    }

    /// Create a new URL loader with default cache directory.
    pub fn with_default_cache(url: impl Into<String>) -> Self {
        let cache_dir = dirs::cache_dir()
            .map(|d| d.join("plugin-system").join("plugins"))
            .unwrap_or_else(|| PathBuf::from("/tmp/plugin-system-cache"));
        Self {
            url: url.into(),
            cache_dir: Some(cache_dir),
        }
    }

    /// Get the URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Get the cache path for this URL.
    pub fn cache_path(&self) -> Option<PathBuf> {
        self.cache_dir.as_ref().map(|cache_dir| {
            let filename = self.url_to_filename();
            cache_dir.join(filename)
        })
    }

    /// Convert URL to a safe filename.
    fn url_to_filename(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.url.hash(&mut hasher);
        let hash = hasher.finish();

        let ext = if cfg!(target_os = "linux") {
            "so"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        };

        format!("plugin_{:016x}.{}", hash, ext)
    }

    /// Download the plugin binary.
    fn download(&self) -> Result<Vec<u8>> {
        log::info!("Downloading plugin from: {}", self.url);

        let response = ureq::get(&self.url)
            .call()
            .map_err(|e| PluginError::DownloadFailed {
                url: self.url.clone(),
                reason: e.to_string(),
            })?;

        let status = response.status();
        if status != 200 {
            return Err(PluginError::HttpError {
                url: self.url.clone(),
                status: status.into(),
            });
        }

        let mut body = response.into_body();
        let bytes = body
            .read_to_vec()
            .map_err(|e| PluginError::DownloadFailed {
                url: self.url.clone(),
                reason: e.to_string(),
            })?;

        log::info!("Downloaded {} bytes from {}", bytes.len(), self.url);
        Ok(bytes)
    }

    /// Save downloaded bytes to cache.
    fn save_to_cache(&self, data: &[u8]) -> Result<PathBuf> {
        if let Some(cache_path) = self.cache_path() {
            if let Some(parent) = cache_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&cache_path, data)?;
            log::info!("Cached plugin to: {}", cache_path.display());
            Ok(cache_path)
        } else {
            Err(PluginError::CacheError(
                "No cache directory configured".to_string(),
            ))
        }
    }

    /// Load from cache if available and not expired.
    fn load_from_cache(&self) -> Option<Vec<u8>> {
        if let Some(cache_path) = self.cache_path() {
            if cache_path.exists() {
                // Check if cache is less than 1 hour old
                if let Ok(metadata) = std::fs::metadata(&cache_path) {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            if elapsed.as_secs() < 3600 {
                                log::info!("Loading plugin from cache: {}", cache_path.display());
                                return std::fs::read(&cache_path).ok();
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(feature = "url-loader")]
impl PluginLoader for UrlLoader {
    fn load(&self) -> Result<Vec<u8>> {
        // Try cache first
        if let Some(cached) = self.load_from_cache() {
            return Ok(cached);
        }

        // Download
        let data = self.download()?;

        // Cache the download
        if let Err(e) = self.save_to_cache(&data) {
            log::warn!("Failed to cache plugin: {}", e);
        }

        Ok(data)
    }

    fn source(&self) -> String {
        self.url.clone()
    }

    fn exists(&self) -> bool {
        // For URLs, we can't really check existence without downloading
        // So we just return true and let the download fail if it doesn't exist
        true
    }
}

/// A loader that loads from multiple sources in order.
pub struct MultiLoader {
    loaders: Vec<Box<dyn PluginLoader>>,
}

impl MultiLoader {
    /// Create a new multi-loader.
    pub fn new() -> Self {
        Self {
            loaders: Vec::new(),
        }
    }

    /// Add a loader source.
    pub fn add_loader(mut self, loader: Box<dyn PluginLoader>) -> Self {
        self.loaders.push(loader);
        self
    }

    /// Add a file loader.
    pub fn add_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.loaders.push(Box::new(FileLoader::new(path)));
        self
    }

    /// Add a URL loader (requires url-loader feature).
    #[cfg(feature = "url-loader")]
    pub fn add_url(mut self, url: impl Into<String>, cache_dir: Option<PathBuf>) -> Self {
        self.loaders.push(Box::new(UrlLoader::new(url, cache_dir)));
        self
    }
}

impl Default for MultiLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginLoader for MultiLoader {
    fn load(&self) -> Result<Vec<u8>> {
        for loader in &self.loaders {
            if loader.exists() {
                match loader.load() {
                    Ok(data) => return Ok(data),
                    Err(e) => {
                        log::warn!("Failed to load from {}: {}", loader.source(), e);
                    }
                }
            }
        }
        Err(PluginError::CacheError(
            "No loader could provide the plugin".to_string(),
        ))
    }

    fn source(&self) -> String {
        self.loaders
            .iter()
            .map(|l| l.source())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn exists(&self) -> bool {
        self.loaders.iter().any(|l| l.exists())
    }
}
