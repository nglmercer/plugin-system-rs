use plugin_system::Plugin;

/// Create a new GreeterPlugin instance.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn plugin_create() -> *mut dyn Plugin {
    let instance = plugin_types::GreeterPlugin::new();
    let boxed: Box<dyn Plugin> = Box::new(instance);
    Box::into_raw(boxed)
}

/// Destroy a GreeterPlugin instance.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn plugin_destroy(ptr: *mut dyn Plugin) {
    if !ptr.is_null() {
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Return GreeterPlugin metadata.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn plugin_metadata() -> plugin_system::PluginMetadata {
    let instance = plugin_types::GreeterPlugin::new();
    instance.metadata()
}
