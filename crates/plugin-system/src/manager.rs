use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::context::PluginContext;
use crate::error::{PluginError, Result};
use crate::handler::{new_shared_command_registry, SharedCommandRegistry};
use crate::loader::PluginLoader;
use crate::manifest::PluginManifest;
use crate::registry::{new_shared_registry, SharedRegistry};
use crate::traits::{Plugin, PluginMetadata};

/// The one extension a plugin binary can have.
///
/// There is no platform matrix any more: a component is the same file on
/// every host, which is most of the point of dropping the native ABIs.
pub const PLUGIN_EXTENSION: &str = "wasm";

struct LoadedPlugin {
    path: PathBuf,
    metadata: PluginMetadata,
    /// Set when the bytes came from a loader rather than from disk, so the
    /// scratch file can be removed on unload.
    temp_path: Option<PathBuf>,
}

pub struct PluginManager {
    registry: SharedRegistry,
    command_registry: SharedCommandRegistry,
    loaded: HashMap<String, LoadedPlugin>,
    /// Created lazily on the first plugin, and shared by all of them.
    wasm_runtime: Option<std::sync::Arc<crate::wasm::WasmRuntime>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            registry: new_shared_registry(),
            command_registry: new_shared_command_registry(),
            loaded: HashMap::new(),
            wasm_runtime: None,
        }
    }

    pub fn registry(&self) -> SharedRegistry {
        self.registry.clone()
    }

    pub fn command_registry(&self) -> SharedCommandRegistry {
        self.command_registry.clone()
    }

    /// Load a plugin from an arbitrary byte source.
    ///
    /// Nothing is written to disk. `dlopen` needed a real file, which is why
    /// this used to spill the bytes into a PID-stamped scratch file and then
    /// chase the leftovers; wasmtime compiles from a byte slice, so the whole
    /// dance is gone. The sidecar manifest is still read from the loader's
    /// source path, since that is where it lives.
    pub fn load_plugin_from_loader(
        &mut self,
        loader: &dyn PluginLoader,
        name: &str,
    ) -> Result<String> {
        let source = loader.source();
        log::info!("Loading plugin '{}' from {}", name, source);

        let bytes = loader.load().map_err(|e| PluginError::PluginLoad {
            name: name.to_string(),
            reason: e.to_string(),
        })?;

        let source_path = PathBuf::from(&source);
        let manifest = detect_manifest(&source_path)?
            .unwrap_or_else(|| PluginManifest::for_component(name));

        self.load_component(&bytes, &manifest, source_path)
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

    /// Load a plugin component from a path.
    pub fn load_plugin(&mut self, path: impl AsRef<Path>) -> Result<String> {
        let path = path.as_ref().to_path_buf();

        log::info!("Loading plugin from {}", path.display());

        // The sidecar manifest carries the capability grants and resource
        // limits. It is optional: a component with no manifest is still a
        // component, it just gets the defaults.
        let manifest = detect_manifest(&path)?.unwrap_or_else(|| {
            PluginManifest::for_component(plugin_stem(&path).as_deref().unwrap_or("plugin"))
        });

        let bytes = std::fs::read(&path).map_err(PluginError::Io)?;
        self.load_component(&bytes, &manifest, path)
    }

    /// Instantiate a component and register it.
    ///
    /// `path` is recorded for `reload_plugin` and for reporting; the bytes are
    /// already in hand, so it is never reopened here.
    fn load_component(
        &mut self,
        bytes: &[u8],
        manifest: &PluginManifest,
        path: PathBuf,
    ) -> Result<String> {
        use crate::wasm::{WasmPlugin, WasmRuntime};

        // The engine is shared across plugins and created on first use.
        let runtime = match &self.wasm_runtime {
            Some(rt) => rt.clone(),
            None => {
                let rt = WasmRuntime::new()?;
                self.wasm_runtime = Some(rt.clone());
                rt
            }
        };

        let plugin = WasmPlugin::load(&runtime, bytes, manifest)?;

        let metadata = plugin.metadata_ref().clone();
        let name = metadata.name.clone();

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

        // Let the plugin reach its peers now that it is about to be live.
        plugin.set_peers(self.command_registry.clone());

        let boxed: Box<dyn Plugin> = Box::new(plugin);
        self.register_and_load(boxed, metadata, path)?;

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

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some(PLUGIN_EXTENSION) {
                continue;
            }
            match self.load_plugin(&path) {
                Ok(name) => loaded.push(name),
                Err(e) => {
                    log::error!("Failed to load {}: {}", path.display(), e);
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

    /// Describe a plugin on disk without instantiating it.
    ///
    /// Reads the sidecar manifest. Asking the binary itself would mean
    /// standing up a wasmtime store and calling `get-metadata`, which is a lot
    /// of machinery for a listing; a plugin that wants to be described without
    /// being run ships a manifest.
    pub fn metadata_from_path(path: impl AsRef<Path>) -> Result<PluginMetadata> {
        let path = path.as_ref();

        match detect_manifest(path)? {
            Some(manifest) => Ok(manifest.into()),
            None => Err(PluginError::PluginLoad {
                name: path.display().to_string(),
                reason: format!(
                    "no sidecar manifest next to {}; a plugin must ship one to be described \
                     without being instantiated",
                    path.display()
                ),
            }),
        }
    }

    pub fn plugin_metadata_from_path(&self, path: impl AsRef<Path>) -> Result<PluginMetadata> {
        Self::metadata_from_path(path)
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
        let dep_names = meta.dependencies_names();
        Ok(crate::plugin_info::PluginInfo {
            name: meta.name,
            version: meta.version,
            authors: meta.authors,
            dependencies: dep_names,
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
            infos.push(crate::plugin_info::PluginInfo {
                name: meta.name,
                version: meta.version,
                authors: meta.authors,
                dependencies: dep_names,
                public_methods: Vec::new(),
            });
        }
        infos
    }

    /// Register an already-constructed plugin, record it as loaded, and run
    /// its `on_load`.
    fn register_and_load(
        &mut self,
        plugin: Box<dyn Plugin>,
        metadata: PluginMetadata,
        path: PathBuf,
    ) -> Result<()> {
        let name = metadata.name.clone();

        if self.loaded.contains_key(&name) {
            self.unload_plugin(&name)?;
        }
        {
            let mut registry = self.write_registry(self.registry.write(), "PluginRegistry")?;
            registry.register(plugin);
        }

        self.loaded.insert(
            name.clone(),
            LoadedPlugin {
                path,
                metadata,
                temp_path: None,
            },
        );

        let ctx = PluginContext::new(self.registry.clone(), self.command_registry.clone());
        let registry = self.read_registry(self.registry.read(), "PluginRegistry")?;
        if let Some(plugin_arc) = registry.get_by_name(&name) {
            let mut plugin = self.write_plugin(plugin_arc.write(), &name)?;
            plugin.on_load(&ctx);
        }

        Ok(())
    }
}

/// Read the sidecar `<plugin>.manifest.json`, if any.
///
/// Returns `Ok(None)` when no manifest is present. That is not an error: the
/// guest's `get-metadata` is the authority on identity either way, and the
/// manifest only adds capability grants and resource limits on top.
fn detect_manifest(plugin_path: &Path) -> Result<Option<PluginManifest>> {
    let stem = match plugin_path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s,
        None => return Ok(None),
    };
    crate::manifest::load_plugin_manifest(plugin_path).map_err(|e| PluginError::PluginLoad {
        name: stem.to_string(),
        reason: format!("invalid manifest: {e}"),
    })
}

/// The plugin name implied by a file path.
///
/// Only a fallback for a component shipped without a manifest: `get-metadata`
/// still decides the real name once the guest is instantiated. Strips the
/// `plugin_`/`plugin-` prefix the build tooling adds, so `plugin_timer.wasm`
/// implies `timer`.
fn plugin_stem(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let name = stem
        .strip_prefix("plugin_")
        .or_else(|| stem.strip_prefix("plugin-"))
        .unwrap_or(stem);
    if name.is_empty() {
        return None;
    }
    Some(name.replace('-', "_"))
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
