use crate::registry::{PluginRegistry, SharedRegistry};

/// Context passed to plugins during `on_load`.
///
/// Provides access to the shared plugin registry so plugins can
/// discover and interact with each other.
pub struct PluginContext {
    pub registry: SharedRegistry,
}

impl PluginContext {
    pub fn new(registry: SharedRegistry) -> Self {
        Self { registry }
    }

    /// Get a read-locked reference to the registry.
    pub fn registry(&self) -> std::sync::RwLockReadGuard<'_, PluginRegistry> {
        self.registry.read().expect("PluginRegistry lock poisoned")
    }

    /// Get a write-locked reference to the registry.
    pub fn registry_mut(&self) -> std::sync::RwLockWriteGuard<'_, PluginRegistry> {
        self.registry.write().expect("PluginRegistry lock poisoned")
    }
}
