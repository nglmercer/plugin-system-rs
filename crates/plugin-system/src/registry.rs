use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::traits::Plugin;

/// A thread-safe registry of loaded plugin instances.
///
/// Plugins are stored by name and can be retrieved by concrete type
/// or by string name. The registry is shared across all plugins via
/// `PluginContext`.
pub struct PluginRegistry {
    plugins: HashMap<String, Arc<RwLock<Box<dyn Plugin>>>>,
    /// Maps type_id → name for type-based lookup.
    type_index: HashMap<std::any::TypeId, String>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            type_index: HashMap::new(),
        }
    }

    /// Register a plugin instance under its declared name.
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let meta = plugin.metadata();
        let name = meta.name.clone();
        let type_id = (*plugin).type_id();
        self.type_index.insert(type_id, name.clone());
        self.plugins.insert(name, Arc::new(RwLock::new(plugin)));
    }

    /// Remove a plugin by name, returning the wrapped instance.
    pub fn unregister(&mut self, name: &str) -> Option<Arc<RwLock<Box<dyn Plugin>>>> {
        if let Some(arc) = self.plugins.remove(name) {
            // Remove type index entry
            if let Ok(p) = arc.read() {
                let type_id = (*p).type_id();
                self.type_index.remove(&type_id);
            }
            Some(arc)
        } else {
            None
        }
    }

    /// Get a reference to a plugin by its concrete type `T`.
    /// Returns `None` if no plugin of that type is loaded.
    pub fn get<T: Plugin + 'static>(&self) -> Option<Arc<RwLock<Box<dyn Plugin>>>> {
        let type_id = std::any::TypeId::of::<T>();
        if let Some(name) = self.type_index.get(&type_id) {
            self.plugins.get(name).cloned()
        } else {
            None
        }
    }

    /// Get a reference to a plugin by name.
    pub fn get_by_name(&self, name: &str) -> Option<Arc<RwLock<Box<dyn Plugin>>>> {
        self.plugins.get(name).cloned()
    }

    /// List all loaded plugin names.
    pub fn plugin_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Check if a plugin is loaded.
    pub fn contains(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }

    /// Number of loaded plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared, thread-safe handle to the plugin registry.
pub type SharedRegistry = Arc<RwLock<PluginRegistry>>;

/// Create a new shared registry.
pub fn new_shared_registry() -> SharedRegistry {
    Arc::new(RwLock::new(PluginRegistry::new()))
}
