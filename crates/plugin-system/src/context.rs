use crate::registry::{PluginRegistry, SharedRegistry};
use std::sync::Arc;

pub struct PluginContext {
    pub registry: SharedRegistry,
}

impl PluginContext {
    pub fn new(registry: SharedRegistry) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> std::sync::RwLockReadGuard<'_, PluginRegistry> {
        self.registry.read().expect("PluginRegistry lock poisoned")
    }

    pub fn registry_mut(&self) -> std::sync::RwLockWriteGuard<'_, PluginRegistry> {
        self.registry.write().expect("PluginRegistry lock poisoned")
    }

    pub fn get_plugin<T: crate::traits::Plugin + 'static>(
        &self,
        name: &str,
    ) -> Option<Arc<std::sync::RwLock<Box<dyn crate::traits::Plugin>>>> {
        let registry = self.registry.read().expect("registry lock poisoned");
        registry.get_by_name(name)
    }

    pub fn with_plugin<R>(
        &self,
        name: &str,
        f: impl FnOnce(&dyn crate::traits::Plugin) -> R,
    ) -> Option<R> {
        let registry = self.registry.read().expect("registry lock poisoned");
        let plugin_arc = registry.get_by_name(name)?;
        let guard = plugin_arc.read().expect("plugin lock poisoned");
        let plugin_ref: &dyn crate::traits::Plugin = &**guard;
        Some(f(plugin_ref))
    }

    pub fn with_plugin_mut<R>(
        &self,
        name: &str,
        f: impl FnOnce(&mut dyn crate::traits::Plugin) -> R,
    ) -> Option<R> {
        let registry = self.registry.read().expect("registry lock poisoned");
        let plugin_arc = registry.get_by_name(name)?;
        let mut guard = plugin_arc.write().expect("plugin lock poisoned");
        let plugin_ref: &mut dyn crate::traits::Plugin = &mut **guard;
        Some(f(plugin_ref))
    }
}
