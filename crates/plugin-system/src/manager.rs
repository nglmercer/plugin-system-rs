use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::context::PluginContext;
use crate::error::{PluginError, Result};
use crate::loader::PluginLoader;
use crate::registry::{new_shared_registry, SharedRegistry};
use crate::traits::{Plugin, PluginMetadata};

/// Raw C ABI function types exported by plugin libraries.
#[allow(improper_ctypes_definitions)]
type PluginCreateFn = unsafe extern "C" fn() -> *mut dyn Plugin;
#[allow(improper_ctypes_definitions)]
type PluginDestroyFn = unsafe extern "C" fn(*mut dyn Plugin);
#[allow(improper_ctypes_definitions)]
type PluginMetadataFn = unsafe extern "C" fn() -> PluginMetadata;

/// Holds a loaded dynamic library and its raw function pointers.
struct PluginLibrary {
    _lib: libloading::Library,
    #[allow(dead_code)]
    create: PluginCreateFn,
    #[allow(dead_code)]
    destroy: PluginDestroyFn,
    metadata_fn: PluginMetadataFn,
}

/// A loaded plugin instance with its associated library handle.
struct LoadedPlugin {
    library: PluginLibrary,
    path: PathBuf,
}

/// Manages the lifecycle of plugins: loading, unloading, and hot-reloading.
pub struct PluginManager {
    registry: SharedRegistry,
    loaded: HashMap<String, LoadedPlugin>,
}

impl PluginManager {
    /// Create a new plugin manager with an empty registry.
    pub fn new() -> Self {
        Self {
            registry: new_shared_registry(),
            loaded: HashMap::new(),
        }
    }

    /// Get a shared reference to the plugin registry.
    pub fn registry(&self) -> SharedRegistry {
        self.registry.clone()
    }

    /// Load a plugin from a `PluginLoader`.
    ///
    /// This method loads the plugin bytes from the loader, writes them
    /// to a temporary file, and then loads the dynamic library from that file.
    ///
    /// # Arguments
    /// * `loader` - The loader to fetch plugin bytes from
    /// * `name` - An identifier for this plugin source (used in error messages)
    pub fn load_plugin_from_loader(
        &mut self,
        loader: &dyn PluginLoader,
        name: &str,
    ) -> Result<String> {
        log::info!("Loading plugin '{}' from {}", name, loader.source());

        // Load bytes from the loader
        let bytes = loader.load().map_err(|e| PluginError::PluginLoad {
            name: name.to_string(),
            reason: e.to_string(),
        })?;

        // Determine the appropriate extension
        let ext = if cfg!(target_os = "linux") {
            "so"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        };

        // Write to a temporary file
        let temp_dir = std::env::temp_dir().join("plugin-system");
        std::fs::create_dir_all(&temp_dir)?;

        let temp_path = temp_dir.join(format!("{}_{}.{}", name, std::process::id(), ext));
        std::fs::write(&temp_path, &bytes)?;

        log::info!("Wrote {} bytes to temp file: {}", bytes.len(), temp_path.display());

        // Load the plugin from the temp file
        let result = self.load_plugin(&temp_path);

        // Clean up temp file (the plugin is now loaded in memory)
        // We don't delete it immediately because the library might need it
        // until it's fully loaded. The OS will clean up temp files.

        result
    }

    /// Load a single plugin from a dynamic library path.
    ///
    /// The library must export:
    /// - `plugin_create() -> *mut dyn Plugin`
    /// - `plugin_destroy(*mut dyn Plugin)`
    /// - `plugin_metadata() -> PluginMetadata`
    pub fn load_plugin(&mut self, path: impl AsRef<Path>) -> Result<String> {
        let path = path.as_ref().to_path_buf();
        let path_display = path.display().to_string();

        log::info!("Loading plugin from {}", path_display);

        // Load the dynamic library
        let lib = unsafe {
            libloading::Library::new(&path).map_err(|e| PluginError::LibraryLoad {
                path: path.clone(),
                reason: e.to_string(),
            })?
        };

        // Resolve symbols
        let create: PluginCreateFn = unsafe {
            *lib.get(b"plugin_create").map_err(|_| PluginError::SymbolNotFound {
                symbol: "plugin_create".to_string(),
            })?
        };

        let destroy: PluginDestroyFn = unsafe {
            *lib.get(b"plugin_destroy").map_err(|_| PluginError::SymbolNotFound {
                symbol: "plugin_destroy".to_string(),
            })?
        };

        let metadata_fn: PluginMetadataFn = unsafe {
            *lib.get(b"plugin_metadata").map_err(|_| PluginError::SymbolNotFound {
                symbol: "plugin_metadata".to_string(),
            })?
        };

        // Get metadata first to check dependencies
        let metadata = unsafe { metadata_fn() };
        let name = metadata.name.clone();

        log::info!("Plugin metadata: {} v{}", name, metadata.version);

        // Check dependencies
        {
            let registry = self.registry.read().expect("registry lock poisoned");
            for dep in &metadata.dependencies {
                if !registry.contains(dep) {
                    return Err(PluginError::MissingDependency {
                        plugin: name.clone(),
                        dependency: dep.clone(),
                    });
                }
            }
        }

        // Create plugin instance
        let raw_instance = unsafe { create() };
        let instance: Box<dyn Plugin> = unsafe { Box::from_raw(raw_instance) };

        // If a plugin with this name already exists, unload it first
        if self.loaded.contains_key(&name) {
            self.unload_plugin(&name)?;
        }

        // Register in the plugin's internal state
        {
            let mut registry = self.registry.write().expect("registry lock poisoned");
            registry.register(instance);
        }

        let plugin_library = PluginLibrary {
            _lib: lib,
            create,
            destroy,
            metadata_fn,
        };

        let loaded_plugin = LoadedPlugin {
            library: plugin_library,
            path: path.clone(),
        };

        self.loaded.insert(name.clone(), loaded_plugin);

        // Call on_load with context
        {
            let ctx = PluginContext::new(self.registry.clone());
            let registry = self.registry.read().expect("registry lock poisoned");
            if let Some(plugin_arc) = registry.get_by_name(&name) {
                if let Ok(mut plugin) = plugin_arc.write() {
                    plugin.on_load(&ctx);
                }
            }
        }

        log::info!("Plugin '{}' loaded successfully", name);
        Ok(name)
    }

    /// Load all plugin libraries from a directory.
    ///
    /// Scans for `.so` (Linux), `.dylib` (macOS), or `.dll` (Windows) files.
    pub fn load_plugins_from_dir(&mut self, dir: impl AsRef<Path>) -> Result<Vec<String>> {
        let dir = dir.as_ref();
        log::info!("Scanning for plugins in {}", dir.display());

        let mut loaded = Vec::new();

        let expected_ext = if cfg!(target_os = "linux") {
            "so"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        };

        if dir.exists() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == expected_ext {
                            match self.load_plugin(&path) {
                                Ok(name) => loaded.push(name),
                                Err(e) => {
                                    log::error!("Failed to load {}: {}", path.display(), e);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            log::warn!("Plugin directory {} does not exist", dir.display());
        }

        Ok(loaded)
    }

    /// Unload a plugin by name.
    ///
    /// Calls `on_unload()` on the plugin, then the destroy function,
    /// then removes it from the registry.
    pub fn unload_plugin(&mut self, name: &str) -> Result<()> {
        log::info!("Unloading plugin '{}'", name);

        // Call on_unload
        {
            let registry = self.registry.read().expect("registry lock poisoned");
            if let Some(plugin_arc) = registry.get_by_name(name) {
                if let Ok(mut plugin) = plugin_arc.write() {
                    plugin.on_unload();
                }
            }
        }

        // Remove from registry
        {
            let mut registry = self.registry.write().expect("registry lock poisoned");
            registry
                .unregister(name)
                .ok_or_else(|| PluginError::PluginNotFound {
                    name: name.to_string(),
                })?;
        }

        // Remove from loaded map (this drops the Library, unloading the .so)
        self.loaded
            .remove(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;

        log::info!("Plugin '{}' unloaded", name);
        Ok(())
    }

    /// Reload a plugin: unload then load from the same path.
    pub fn reload_plugin(&mut self, name: &str) -> Result<()> {
        let path = self
            .loaded
            .get(name)
            .map(|p| p.path.clone())
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;

        log::info!("Reloading plugin '{}' from {}", name, path.display());

        self.unload_plugin(name)?;
        self.load_plugin(path)?;

        Ok(())
    }

    /// Get a list of all loaded plugin names.
    pub fn plugin_names(&self) -> Vec<String> {
        self.registry
            .read()
            .expect("registry lock poisoned")
            .plugin_names()
    }

    /// Check if a plugin is loaded.
    pub fn is_loaded(&self, name: &str) -> bool {
        self.registry
            .read()
            .expect("registry lock poisoned")
            .contains(name)
    }

    /// Get the path a plugin was loaded from.
    pub fn plugin_path(&self, name: &str) -> Option<PathBuf> {
        self.loaded.get(name).map(|p| p.path.clone())
    }

    /// Get metadata for a loaded plugin without instantiating it.
    pub fn plugin_metadata(&self, name: &str) -> Option<PluginMetadata> {
        self.loaded
            .get(name)
            .map(|p| unsafe { (p.library.metadata_fn)() })
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
