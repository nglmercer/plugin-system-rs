use std::any::Any;

/// Metadata describing a plugin's identity and requirements.
#[repr(C)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub dependencies: Vec<String>,
}

/// The core trait that all plugins must implement.
///
/// Plugins are loaded as dynamic libraries and instantiated via
/// the exported `plugin_create` symbol.
pub trait Plugin: Any + Send + Sync {
    /// Returns metadata about this plugin.
    fn metadata(&self) -> PluginMetadata;

    /// Called when the plugin is loaded and registered.
    /// Use `ctx` to access other loaded plugins.
    fn on_load(&mut self, ctx: &crate::context::PluginContext);

    /// Called when the plugin is being unloaded.
    /// Clean up any resources here.
    fn on_unload(&mut self);

    /// Handle a command from the host or another plugin.
    ///
    /// This is the primary way for the host to interact with plugins.
    /// The command string format is: `"method arg1 arg2 ..."`
    /// The return value is the result string, or an error message.
    ///
    /// # Default Implementation
    /// Returns an error message for unknown commands.
    fn handle_command(&mut self, command: &str) -> String {
        format!("ERROR: Unknown command '{}'", command)
    }
}

impl dyn Plugin {
    /// Downcast a `&dyn Plugin` to a concrete plugin type.
    pub fn downcast_ref<T: Plugin + 'static>(&self) -> Option<&T> {
        self_any(self).downcast_ref::<T>()
    }

    /// Downcast a `&mut dyn Plugin` to a concrete plugin type.
    pub fn downcast_mut<T: Plugin + 'static>(&mut self) -> Option<&mut T> {
        self_any_mut(self).downcast_mut::<T>()
    }
}

fn self_any(p: &dyn Plugin) -> &dyn Any {
    p
}

fn self_any_mut(p: &mut dyn Plugin) -> &mut dyn Any {
    p
}
