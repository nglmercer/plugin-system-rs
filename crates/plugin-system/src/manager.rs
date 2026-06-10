use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::context::PluginContext;
use crate::error::{PluginError, Result};
use crate::loader::{FileLoader, PluginLoader};
use crate::registry::{new_shared_registry, SharedRegistry};
use crate::traits::{Plugin, PluginMetadata};

#[allow(improper_ctypes_definitions)]
type PluginCreateFn = unsafe extern "C" fn() -> *mut dyn Plugin;
#[allow(improper_ctypes_definitions)]
type PluginDestroyFn = unsafe extern "C" fn(*mut dyn Plugin);
#[allow(improper_ctypes_definitions)]
type PluginMetadataFn = unsafe extern "C" fn() -> PluginMetadata;

struct PluginLibrary {
    _lib: libloading::Library,
    #[allow(dead_code)]
    create: PluginCreateFn,
    #[allow(dead_code)]
    destroy: PluginDestroyFn,
    metadata_fn: PluginMetadataFn,
}

struct LoadedPlugin {
    library: PluginLibrary,
    path: PathBuf,
}

pub struct PluginManager {
    registry: SharedRegistry,
    loaded: HashMap<String, LoadedPlugin>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            registry: new_shared_registry(),
            loaded: HashMap::new(),
        }
    }

    pub fn registry(&self) -> SharedRegistry {
        self.registry.clone()
    }

    pub fn load_plugin_from_loader(
        &mut self,
        loader: &dyn PluginLoader,
        name: &str,
    ) -> Result<String> {
        log::info!("Loading plugin '{}' from {}", name, loader.source());

        let bytes = loader.load().map_err(|e| PluginError::PluginLoad {
            name: name.to_string(),
            reason: e.to_string(),
        })?;

        let ext = if cfg!(target_os = "linux") {
            "so"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        };

        let temp_dir = std::env::temp_dir().join("plugin-system");
        std::fs::create_dir_all(&temp_dir)?;

        let temp_path = temp_dir.join(format!("{}_{}.{}", name, std::process::id(), ext));
        std::fs::write(&temp_path, &bytes)?;

        log::info!(
            "Wrote {} bytes to temp file: {}",
            bytes.len(),
            temp_path.display()
        );

        self.load_plugin(&temp_path)
    }

    pub fn load_plugin(&mut self, path: impl AsRef<Path>) -> Result<String> {
        let path = path.as_ref().to_path_buf();
        let path_display = path.display().to_string();

        log::info!("Loading plugin from {}", path_display);

        let lib = unsafe {
            libloading::Library::new(&path).map_err(|e| PluginError::LibraryLoad {
                path: path.clone(),
                reason: e.to_string(),
            })?
        };

        let create: PluginCreateFn = unsafe {
            *lib.get(b"plugin_create")
                .map_err(|_| PluginError::SymbolNotFound {
                    symbol: "plugin_create".to_string(),
                })?
        };

        let destroy: PluginDestroyFn = unsafe {
            *lib.get(b"plugin_destroy")
                .map_err(|_| PluginError::SymbolNotFound {
                    symbol: "plugin_destroy".to_string(),
                })?
        };

        let metadata_fn: PluginMetadataFn = unsafe {
            *lib.get(b"plugin_metadata")
                .map_err(|_| PluginError::SymbolNotFound {
                    symbol: "plugin_metadata".to_string(),
                })?
        };

        let metadata = unsafe { metadata_fn() };
        let name = metadata.name.clone();

        log::info!("Plugin metadata: {} v{}", name, metadata.version);

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

        let raw_instance = unsafe { create() };
        let instance: Box<dyn Plugin> = unsafe { Box::from_raw(raw_instance) };

        if self.loaded.contains_key(&name) {
            self.unload_plugin(&name)?;
        }

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

    pub fn load_plugins_from_dir(&mut self, dir: impl AsRef<Path>) -> Result<Vec<String>> {
        let dir = dir.as_ref();
        log::info!("Scanning for plugins in {}", dir.display());

        let mut loaded = Vec::new();

        if !dir.exists() {
            log::warn!("Plugin directory {} does not exist", dir.display());
            return Ok(loaded);
        }

        let expected_ext = if cfg!(target_os = "linux") {
            "so"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else if cfg!(target_os = "windows") {
            "dll"
        } else {
            "so"
        };

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == expected_ext {
                        let loader = FileLoader::new(&path);
                        let name = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        match self.load_plugin_from_loader(&loader, &name) {
                            Ok(name) => loaded.push(name),
                            Err(e) => {
                                log::error!("Failed to load {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }

        Ok(loaded)
    }

    pub fn load_plugins_from_loaders(
        &mut self,
        loaders: &[(String, Box<dyn PluginLoader>)],
    ) -> Result<Vec<String>> {
        let mut loaded = Vec::new();

        for (name, loader) in loaders {
            match self.load_plugin_from_loader(loader.as_ref(), name) {
                Ok(name) => loaded.push(name),
                Err(e) => {
                    log::error!("Failed to load plugin '{}': {}", name, e);
                }
            }
        }

        Ok(loaded)
    }

    pub fn unload_plugin(&mut self, name: &str) -> Result<()> {
        log::info!("Unloading plugin '{}'", name);

        {
            let registry = self.registry.read().expect("registry lock poisoned");
            if let Some(plugin_arc) = registry.get_by_name(name) {
                if let Ok(mut plugin) = plugin_arc.write() {
                    plugin.on_unload();
                }
            }
        }

        {
            let mut registry = self.registry.write().expect("registry lock poisoned");
            registry
                .unregister(name)
                .ok_or_else(|| PluginError::PluginNotFound {
                    name: name.to_string(),
                })?;
        }

        self.loaded
            .remove(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;

        log::info!("Plugin '{}' unloaded", name);
        Ok(())
    }

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

    pub fn plugin_names(&self) -> Vec<String> {
        self.registry
            .read()
            .expect("registry lock poisoned")
            .plugin_names()
    }

    pub fn is_loaded(&self, name: &str) -> bool {
        self.registry
            .read()
            .expect("registry lock poisoned")
            .contains(name)
    }

    pub fn plugin_path(&self, name: &str) -> Option<PathBuf> {
        self.loaded.get(name).map(|p| p.path.clone())
    }

    pub fn plugin_metadata(&self, name: &str) -> Option<PluginMetadata> {
        self.loaded
            .get(name)
            .map(|p| unsafe { (p.library.metadata_fn)() })
    }

    pub fn call_plugin(&self, name: &str, command: &str) -> Result<String> {
        let registry = self.registry.read().expect("registry lock poisoned");
        let plugin_arc = registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;

        let mut plugin = plugin_arc.write().expect("plugin lock poisoned");
        Ok(plugin.handle_command(command))
    }

    pub fn call_plugins(&self, command: &str) -> HashMap<String, String> {
        let registry = self.registry.read().expect("registry lock poisoned");
        let mut results = HashMap::new();

        for name in registry.plugin_names() {
            if let Some(plugin_arc) = registry.get_by_name(&name) {
                if let Ok(mut plugin) = plugin_arc.write() {
                    results.insert(name, plugin.handle_command(command));
                }
            }
        }

        results
    }

    pub fn plugin_commands(&self, name: &str) -> Option<String> {
        self.call_plugin(name, "help").ok()
    }

    pub fn with_plugin<R>(&self, name: &str, f: impl FnOnce(&dyn Plugin) -> R) -> Result<R> {
        let registry = self.registry.read().expect("registry lock poisoned");
        let plugin_arc = registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;
        let guard = plugin_arc.read().expect("plugin lock poisoned");
        let plugin_ref: &dyn Plugin = &**guard;
        Ok(f(plugin_ref))
    }

    pub fn with_plugin_mut<R>(
        &self,
        name: &str,
        f: impl FnOnce(&mut dyn Plugin) -> R,
    ) -> Result<R> {
        let registry = self.registry.read().expect("registry lock poisoned");
        let plugin_arc = registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;
        let mut guard = plugin_arc.write().expect("plugin lock poisoned");
        let plugin_ref: &mut dyn Plugin = &mut **guard;
        Ok(f(plugin_ref))
    }

    pub fn get_plugin_arc(
        &self,
        name: &str,
    ) -> Result<std::sync::Arc<std::sync::RwLock<Box<dyn Plugin>>>> {
        let registry = self.registry.read().expect("registry lock poisoned");
        registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })
    }

    pub fn plugins_with_interface(&self, interface_name: &str) -> Vec<String> {
        let registry = self.registry.read().expect("registry lock poisoned");
        registry.get_by_interface(interface_name)
    }

    pub fn list_all_interfaces(&self) -> HashMap<String, Vec<String>> {
        let registry = self.registry.read().expect("registry lock poisoned");
        registry.list_interfaces()
    }

    pub fn call_interface_method<R>(
        &self,
        plugin_name: &str,
        interface_name: &str,
        f: impl FnOnce(&dyn Plugin) -> R,
    ) -> Result<R> {
        let registry = self.registry.read().expect("registry lock poisoned");
        let plugin_arc = registry
            .get_by_name(plugin_name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: plugin_name.to_string(),
            })?;
        let guard = plugin_arc.read().expect("plugin lock poisoned");
        let plugin_ref: &dyn Plugin = &**guard;

        if !plugin_ref.has_interface(interface_name) {
            return Err(PluginError::PluginNotFound {
                name: format!(
                    "Plugin '{}' does not implement interface '{}'",
                    plugin_name, interface_name
                ),
            });
        }

        Ok(f(plugin_ref))
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
