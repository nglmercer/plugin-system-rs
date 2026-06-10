/// Macro to reduce boilerplate when defining plugins.
///
/// # Usage
///
/// ```ignore
/// use plugin_system::define_plugin;
/// use plugin_system::traits::{Plugin, PluginMetadata};
/// use plugin_system::context::PluginContext;
///
/// struct MyPlugin {
///     data: String,
/// }
///
/// impl Plugin for MyPlugin {
///     fn metadata(&self) -> PluginMetadata {
///         PluginMetadata {
///             name: "my_plugin".to_string(),
///             version: "0.1.0".to_string(),
///             authors: vec!["Author".to_string()],
///             dependencies: vec![],
///         }
///     }
///     fn on_load(&mut self, ctx: &PluginContext) {}
///     fn on_unload(&mut self) {}
/// }
///
/// define_plugin!(MyPlugin);
/// ```
///
/// This generates the `#[no_mangle] extern "C"` functions required
/// for the host to load the plugin via `libloading`.
#[macro_export]
macro_rules! define_plugin {
    ($plugin_type:ty) => {
        /// Create a new instance of the plugin.
        /// Called by the host application via `libloading`.
        #[no_mangle]
        pub extern "C" fn plugin_create() -> *mut dyn $crate::traits::Plugin {
            let instance = <$plugin_type>::new();
            let boxed: Box<dyn $crate::traits::Plugin> = Box::new(instance);
            Box::into_raw(boxed)
        }

        /// Destroy a plugin instance.
        /// Called by the host application to safely deallocate across heaps.
        #[no_mangle]
        pub extern "C" fn plugin_destroy(ptr: *mut dyn $crate::traits::Plugin) {
            if !ptr.is_null() {
                unsafe {
                    drop(Box::from_raw(ptr));
                }
            }
        }

        /// Return plugin metadata without instantiating the plugin.
        /// Called by the host for dependency/version checks.
        #[no_mangle]
        pub extern "C" fn plugin_metadata() -> $crate::traits::PluginMetadata {
            let instance = <$plugin_type>::new();
            instance.metadata()
        }
    };
}

/// Helper to define metadata on a struct-level.
///
/// # Usage
///
/// ```ignore
/// plugin_metadata! {
///     name: "my_plugin",
///     version: "0.1.0",
///     authors: ["Author"],
///     dependencies: ["other_plugin"],
/// }
/// ```
#[macro_export]
macro_rules! plugin_metadata {
    (
        name: $name:expr,
        version: $version:expr,
        authors: [$($author:expr),* $(,)?],
        dependencies: [$($dep:expr),* $(,)?]
    ) => {
        $crate::traits::PluginMetadata {
            name: $name.to_string(),
            version: $version.to_string(),
            authors: vec![$($author.to_string()),*],
            dependencies: vec![$($dep.to_string()),*],
        }
    };
}
