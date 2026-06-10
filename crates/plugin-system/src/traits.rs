use std::any::Any;

#[repr(C)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub dependencies: Vec<String>,
}

pub trait Plugin: Any + Send + Sync {
    fn metadata(&self) -> PluginMetadata;

    fn on_load(&mut self, ctx: &crate::context::PluginContext);

    fn on_unload(&mut self);

    fn plugin_type_name(&self) -> &'static str;

    fn handle_command(&mut self, command: &str) -> String {
        format!("ERROR: Unknown command '{}'", command)
    }
}

impl dyn Plugin {
    pub fn downcast_ref<T: Plugin + 'static>(&self) -> Option<&T> {
        if self.plugin_type_name() == std::any::type_name::<T>() {
            let ptr = self as *const dyn Plugin as *const T;
            Some(unsafe { &*ptr })
        } else {
            None
        }
    }

    pub fn downcast_mut<T: Plugin + 'static>(&mut self) -> Option<&mut T> {
        if self.plugin_type_name() == std::any::type_name::<T>() {
            let ptr = self as *mut dyn Plugin as *mut T;
            Some(unsafe { &mut *ptr })
        } else {
            None
        }
    }
}
