use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::traits::Plugin;

pub struct PluginRegistry {
    plugins: HashMap<String, Arc<RwLock<Box<dyn Plugin>>>>,
    type_index: HashMap<std::any::TypeId, String>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            type_index: HashMap::new(),
        }
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let meta = plugin.metadata();
        let name = meta.name.clone();
        let type_id = (*plugin).type_id();

        self.type_index.insert(type_id, name.clone());
        self.plugins.insert(name, Arc::new(RwLock::new(plugin)));
    }

    pub fn unregister(&mut self, name: &str) -> Option<Arc<RwLock<Box<dyn Plugin>>>> {
        if let Some(arc) = self.plugins.remove(name) {
            self.type_index.retain(|_, n| n != name);
            Some(arc)
        } else {
            None
        }
    }

    pub fn get<T: Plugin + 'static>(&self) -> Option<Arc<RwLock<Box<dyn Plugin>>>> {
        let type_id = std::any::TypeId::of::<T>();
        if let Some(name) = self.type_index.get(&type_id) {
            self.plugins.get(name).cloned()
        } else {
            None
        }
    }

    pub fn get_by_name(&self, name: &str) -> Option<Arc<RwLock<Box<dyn Plugin>>>> {
        self.plugins.get(name).cloned()
    }

    pub fn plugin_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    pub fn iter_plugins(&self) -> impl Iterator<Item = (&String, &Arc<RwLock<Box<dyn Plugin>>>)> {
        self.plugins.iter()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedRegistry = Arc<RwLock<PluginRegistry>>;

pub fn new_shared_registry() -> SharedRegistry {
    Arc::new(RwLock::new(PluginRegistry::new()))
}
