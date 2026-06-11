use std::any::Any;

use crate::error::Result;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginDependency {
    pub name: String,
    pub version_req: String,
}

#[repr(C)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub dependencies: Vec<PluginDependency>,
}

impl PluginMetadata {
    pub fn dependencies_names(&self) -> Vec<String> {
        self.dependencies.iter().map(|d| d.name.clone()).collect()
    }
}

pub trait Plugin: Any + Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    fn on_load(&mut self, ctx: &crate::context::PluginContext);

    fn on_unload(&mut self);

    fn plugin_type_name(&self) -> &'static str;

    fn interface_ids(&self) -> Vec<&'static str> {
        Vec::new()
    }

    fn interface_data(&self) -> Option<serde_json::Value> {
        None
    }

    fn simulate_keys(&mut self, _keys: &[String]) -> Result<()> {
        Err(crate::error::PluginError::PluginNotFound {
            name: "simulate_keys not supported".to_string(),
        })
    }

    fn listen_for_combo(&self, _timeout_ms: u64) -> Result<String> {
        Err(crate::error::PluginError::PluginNotFound {
            name: "listen_for_combo not supported".to_string(),
        })
    }

    fn reset_recording_state(&self) {}
}

impl dyn Plugin {
    pub fn downcast_ref<T: Plugin + 'static>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }

    pub fn downcast_mut<T: Plugin + 'static>(&mut self) -> Option<&mut T> {
        (self as &mut dyn Any).downcast_mut::<T>()
    }

    pub fn has_interface(&self, interface_name: &str) -> bool {
        self.interface_ids().contains(&interface_name)
    }
}
