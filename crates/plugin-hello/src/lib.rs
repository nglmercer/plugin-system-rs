use plugin_system::Plugin;

/// Create a new HelloPlugin instance.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn plugin_create() -> *mut dyn Plugin {
    let instance = plugin_types::HelloPlugin::new();
    let boxed: Box<dyn Plugin> = Box::new(instance);
    Box::into_raw(boxed)
}

/// Destroy a HelloPlugin instance.
///
/// # Safety
///
/// This function is called via FFI and must receive a valid pointer
/// previously returned by `plugin_create`.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub unsafe extern "C" fn plugin_destroy(ptr: *mut dyn Plugin) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr));
    }
}

/// Return HelloPlugin metadata.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn plugin_metadata() -> plugin_system::PluginMetadata {
    let instance = plugin_types::HelloPlugin::new();
    instance.metadata()
}
