use std::any::Any;

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

/// Result type for command handlers.
pub type CommandResult = Result<serde_json::Value, String>;

/// Convert a CommandResult to Option<Value> for backward compatibility.
pub fn command_to_json(result: CommandResult) -> Option<serde_json::Value> {
    match result {
        Ok(value) => Some(value),
        Err(e) => Some(serde_json::json!({"ok": false, "error": e})),
    }
}

pub trait Plugin: Any + Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    fn on_load(&mut self, ctx: &crate::context::PluginContext);

    fn on_unload(&mut self);

    fn plugin_type_name(&self) -> &'static str;

    fn handle_command(
        &mut self,
        _method: &str,
        _args: serde_json::Value,
    ) -> Option<serde_json::Value> {
        None
    }

    fn interface_ids(&self) -> Vec<&'static str> {
        Vec::new()
    }

    fn interface_data(&self) -> Option<serde_json::Value> {
        None
    }
}

impl dyn Plugin {
    pub fn downcast_ref<T: Plugin + 'static>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref::<T>()
    }

    pub fn downcast_mut<T: Plugin + 'static>(&mut self) -> Option<&mut T> {
        (self as &mut dyn Any).downcast_mut::<T>()
    }
}
