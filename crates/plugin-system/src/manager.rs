use std::collections::HashMap;
use std::ffi::CStr;
use std::path::{Path, PathBuf};

use crate::context::PluginContext;
use crate::error::{PluginError, Result};
use crate::loader::{FileLoader, PluginLoader};
use crate::registry::{new_shared_registry, SharedRegistry};
use crate::traits::{Plugin, PluginMetadata};

type PluginCreateFn = unsafe extern "C" fn() -> *mut ();
type PluginDestroyFn = unsafe extern "C" fn(*mut ());
type PluginMetadataJsonFn = unsafe extern "C" fn() -> *mut std::ffi::c_char;
type PluginFreeStringFn = unsafe extern "C" fn(*mut std::ffi::c_char);

struct LoadedPlugin {
    _lib: libloading::Library,
    path: PathBuf,
    metadata: PluginMetadata,
    temp_path: Option<PathBuf>,
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

        let actual_name = self.load_plugin(&temp_path)?;
        if let Some(loaded) = self.loaded.get_mut(&actual_name) {
            if loaded.temp_path.is_none() {
                loaded.temp_path = Some(temp_path.clone());
            }
        }
        Ok(actual_name)
    }

    fn read_registry<'a>(
        &'a self,
        guard: std::sync::LockResult<
            std::sync::RwLockReadGuard<'a, crate::registry::PluginRegistry>,
        >,
        lock_name: &str,
    ) -> Result<std::sync::RwLockReadGuard<'a, crate::registry::PluginRegistry>> {
        match guard {
            Ok(reg) => Ok(reg),
            Err(poisoned) => {
                let reg = poisoned.into_inner();
                log::error!("{} poisoned; recovering with current state", lock_name);
                Ok(reg)
            }
        }
    }

    fn write_registry<'a>(
        &'a self,
        guard: std::sync::LockResult<
            std::sync::RwLockWriteGuard<'a, crate::registry::PluginRegistry>,
        >,
        lock_name: &str,
    ) -> Result<std::sync::RwLockWriteGuard<'a, crate::registry::PluginRegistry>> {
        match guard {
            Ok(reg) => Ok(reg),
            Err(poisoned) => {
                let reg = poisoned.into_inner();
                log::error!("{} poisoned; recovering with current state", lock_name);
                Ok(reg)
            }
        }
    }

    fn write_plugin<'a>(
        &'a self,
        guard: std::sync::LockResult<
            std::sync::RwLockWriteGuard<'a, Box<dyn crate::traits::Plugin>>,
        >,
        plugin_name: &str,
    ) -> Result<std::sync::RwLockWriteGuard<'a, Box<dyn crate::traits::Plugin>>> {
        match guard {
            Ok(plugin) => Ok(plugin),
            Err(poisoned) => {
                let plugin = poisoned.into_inner();
                log::error!(
                    "Plugin '{}' lock poisoned; recovering with current state",
                    plugin_name
                );
                Ok(plugin)
            }
        }
    }

    fn read_plugin<'a>(
        &'a self,
        guard: std::sync::LockResult<
            std::sync::RwLockReadGuard<'a, Box<dyn crate::traits::Plugin>>,
        >,
        plugin_name: &str,
    ) -> Result<std::sync::RwLockReadGuard<'a, Box<dyn crate::traits::Plugin>>> {
        match guard {
            Ok(plugin) => Ok(plugin),
            Err(poisoned) => {
                let plugin = poisoned.into_inner();
                log::error!(
                    "Plugin '{}' lock poisoned; recovering with current state",
                    plugin_name
                );
                Ok(plugin)
            }
        }
    }

    fn remove_temp_path(&self, name: &str) {
        if let Some(temp_path) = self
            .loaded
            .get(name)
            .and_then(|loaded| loaded.temp_path.clone())
        {
            let _ = std::fs::remove_file(&temp_path);
            log::debug!("Removed temp plugin file: {}", temp_path.display());
        }
    }

    fn load_metadata(lib: &libloading::Library, path: &Path) -> Result<PluginMetadata> {
        if let Some(manifest) = crate::manifest::load_manifest(path).map_err(PluginError::Io)? {
            return Ok(manifest.into());
        }

        let metadata_json_fn: PluginMetadataJsonFn = unsafe {
            *lib.get(b"plugin_metadata_json")
                .map_err(|_| PluginError::SymbolNotFound {
                    symbol: "plugin_metadata_json".to_string(),
                })?
        };
        let free_string_fn: PluginFreeStringFn = unsafe {
            *lib.get(b"plugin_free_string")
                .map_err(|_| PluginError::SymbolNotFound {
                    symbol: "plugin_free_string".to_string(),
                })?
        };

        let ptr = unsafe { metadata_json_fn() };
        if ptr.is_null() {
            return Err(PluginError::PluginLoad {
                name: "metadata".to_string(),
                reason: "plugin_metadata_json returned null".to_string(),
            });
        }

        let json = unsafe { CStr::from_ptr(ptr) }
            .to_str()
            .map_err(|e| PluginError::PluginLoad {
                name: "metadata".to_string(),
                reason: e.to_string(),
            })?
            .to_string();

        unsafe { free_string_fn(ptr) };

        serde_json::from_str(&json).map_err(|e| PluginError::PluginLoad {
            name: "metadata".to_string(),
            reason: e.to_string(),
        })
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

        let _destroy: PluginDestroyFn = unsafe {
            *lib.get(b"plugin_destroy")
                .map_err(|_| PluginError::SymbolNotFound {
                    symbol: "plugin_destroy".to_string(),
                })?
        };

        let metadata = Self::load_metadata(&lib, &path)?;
        let name = metadata.name.clone();

        log::info!("Plugin metadata: {} v{}", name, metadata.version);

        {
            let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
            for dep in &metadata.dependencies {
                if !registry.contains(dep.name.as_str()) {
                    return Err(PluginError::MissingDependency {
                        plugin: name.clone(),
                        dependency: dep.name.clone(),
                    });
                }
            }
        }

        let found_version = self
            .loaded
            .get(&name)
            .map(|p| p.metadata.version.clone())
            .unwrap_or_default();
        if !found_version.is_empty() {
            for dep in &metadata.dependencies {
                let req = match semver::VersionReq::parse(&dep.version_req) {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(PluginError::InvalidSemverRequirement {
                            name: name.clone(),
                            reason: format!("{}: {}", dep.name, e),
                        });
                    }
                };
                let found = match semver::Version::parse(&found_version) {
                    Ok(v) => v,
                    Err(e) => {
                        return Err(PluginError::InvalidSemverRequirement {
                            name: name.clone(),
                            reason: format!("{}: {}", found_version, e),
                        });
                    }
                };
                if !req.matches(&found) {
                    return Err(PluginError::VersionIncompatible {
                        name: name.clone(),
                        required: dep.version_req.clone(),
                        found: found_version.clone(),
                    });
                }
            }
        }

        let raw_instance = unsafe { create() };
        let boxed: Box<Box<dyn Plugin>> =
            unsafe { Box::from_raw(raw_instance as *mut Box<dyn Plugin>) };
        let instance: Box<dyn Plugin> = *boxed;

        if self.loaded.contains_key(&name) {
            self.unload_plugin(&name)?;
        }

        {
            let mut registry = self.write_registry(self.registry.write(), "PluginRegistry")?;
            registry.register(instance);
        }

        let loaded_plugin = LoadedPlugin {
            _lib: lib,
            path: path.clone(),
            metadata,
            temp_path: None,
        };

        self.loaded.insert(name.clone(), loaded_plugin);

        {
            let ctx = PluginContext::new(self.registry.clone());
            let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
            if let Some(plugin_arc) = registry.get_by_name(&name) {
                let mut plugin = self.write_plugin(plugin_arc.write(), &name)?;
                plugin.on_load(&ctx);
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
            let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
            if let Some(plugin_arc) = registry.get_by_name(name) {
                let mut plugin = self.write_plugin(plugin_arc.write(), name)?;
                plugin.on_unload();
            }
        }

        {
            let mut registry = self.write_registry(self.registry.write(), "PluginRegistry")?;
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

        self.remove_temp_path(name);

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
        let registry = self.registry.read().ok();
        registry.map(|reg| reg.plugin_names()).unwrap_or_default()
    }

    pub fn is_loaded(&self, name: &str) -> bool {
        let registry = self.registry.read().ok();
        registry.map(|reg| reg.contains(name)).unwrap_or(false)
    }

    pub fn plugin_path(&self, name: &str) -> Option<PathBuf> {
        self.loaded.get(name).map(|p| p.path.clone())
    }

    pub fn plugin_metadata(&self, name: &str) -> Option<PluginMetadata> {
        self.loaded.get(name).map(|p| p.metadata.clone())
    }

    pub fn with_plugin<R>(&self, name: &str, f: impl FnOnce(&dyn Plugin) -> R) -> Result<R> {
        let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
        let plugin_arc = registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;
        let guard = self.read_plugin(plugin_arc.read(), name)?;
        let plugin_ref: &dyn Plugin = &**guard;
        Ok(f(plugin_ref))
    }

    pub fn with_plugin_mut<R>(
        &self,
        name: &str,
        f: impl FnOnce(&mut dyn Plugin) -> R,
    ) -> Result<R> {
        let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
        let plugin_arc = registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;
        let mut guard = self.write_plugin(plugin_arc.write(), name)?;
        let plugin_ref: &mut dyn Plugin = &mut **guard;
        Ok(f(plugin_ref))
    }

    pub fn get_plugin_arc(
        &self,
        name: &str,
    ) -> Result<std::sync::Arc<std::sync::RwLock<Box<dyn Plugin>>>> {
        let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
        registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })
    }

    pub fn plugins_with_interface(&self, interface_name: &str) -> Vec<String> {
        let registry = self.registry.read().ok();
        registry
            .map(|reg| reg.get_by_interface(interface_name))
            .unwrap_or_default()
    }

    pub fn list_all_interfaces(&self) -> HashMap<String, Vec<String>> {
        let registry = self.registry.read().ok();
        registry
            .map(|reg| reg.list_interfaces())
            .unwrap_or_default()
    }

    pub fn call_interface_method<R>(
        &self,
        plugin_name: &str,
        interface_name: &str,
        f: impl FnOnce(&dyn Plugin) -> R,
    ) -> Result<R> {
        let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
        let plugin_arc =
            registry
                .get_by_name(plugin_name)
                .ok_or_else(|| PluginError::PluginNotFound {
                    name: plugin_name.to_string(),
                })?;
        let guard = self.read_plugin(plugin_arc.read(), plugin_name)?;
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

    pub fn get_plugin_info(&self, name: &str) -> Result<crate::plugin_info::PluginInfo> {
        let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
        let plugin_arc = registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;
        let guard = self.read_plugin(plugin_arc.read(), name)?;
        let plugin_ref: &dyn Plugin = &**guard;
        let meta = plugin_ref.metadata();
        let interfaces = plugin_ref.interface_ids();
        let dep_names = meta.dependencies_names();
        Ok(crate::plugin_info::PluginInfo {
            name: meta.name,
            version: meta.version,
            authors: meta.authors,
            dependencies: dep_names,
            interfaces: interfaces.into_iter().map(|s| s.to_string()).collect(),
            public_methods: Vec::new(),
        })
    }

    pub fn call_plugin_result(
        &self,
        name: &str,
        f: impl FnOnce(&dyn Plugin) -> crate::plugin_info::PluginResult,
    ) -> Result<crate::plugin_info::PluginResult> {
        self.with_plugin(name, f)
    }

    pub fn get_all_plugin_info(&self) -> Vec<crate::plugin_info::PluginInfo> {
        let registry = match self.registry.read() {
            Ok(r) => r,
            Err(poisoned) => {
                log::error!("PluginRegistry poisoned while listing plugins");
                poisoned.into_inner()
            }
        };
        let mut infos = Vec::new();
        for (plugin_name, plugin_arc) in registry.iter_plugins() {
            let guard = match plugin_arc.read() {
                Ok(p) => p,
                Err(poisoned) => {
                    log::error!("Plugin '{}' lock poisoned while listing", plugin_name);
                    poisoned.into_inner()
                }
            };
            let plugin_ref: &dyn Plugin = &**guard;
            let meta = plugin_ref.metadata();
            let dep_names = meta.dependencies_names();
            let interfaces = plugin_ref.interface_ids();
            infos.push(crate::plugin_info::PluginInfo {
                name: meta.name,
                version: meta.version,
                authors: meta.authors,
                dependencies: dep_names,
                interfaces: interfaces.into_iter().map(|s| s.to_string()).collect(),
                public_methods: Vec::new(),
            });
        }
        infos
    }

    pub fn has_interface(&self, plugin_name: &str, interface_name: &str) -> bool {
        let registry = match self.registry.read() {
            Ok(r) => r,
            Err(poisoned) => {
                log::error!("PluginRegistry poisoned in has_interface");
                poisoned.into_inner()
            }
        };
        if let Some(plugin_arc) = registry.get_by_name(plugin_name) {
            let guard = match plugin_arc.read() {
                Ok(p) => p,
                Err(poisoned) => {
                    log::error!("Plugin '{}' lock poisoned in has_interface", plugin_name);
                    poisoned.into_inner()
                }
            };
            let plugin_ref: &dyn Plugin = &**guard;
            plugin_ref.has_interface(interface_name)
        } else {
            false
        }
    }

    pub fn get_plugin_interfaces(&self, name: &str) -> Result<Vec<String>> {
        let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
        let plugin_arc = registry
            .get_by_name(name)
            .ok_or_else(|| PluginError::PluginNotFound {
                name: name.to_string(),
            })?;
        let guard = self.read_plugin(plugin_arc.read(), name)?;
        let plugin_ref: &dyn Plugin = &**guard;
        Ok(plugin_ref
            .interface_ids()
            .into_iter()
            .map(|s| s.to_string())
            .collect())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
