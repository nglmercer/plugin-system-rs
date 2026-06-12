# plugin-macros

Proc-macro crate that generates FFI glue code for `plugin-system` plugins. These macros produce the `extern "C"` functions that `PluginManager` looks up when loading a shared library.

**You normally don't depend on this crate directly.** Use `plugin-system` which re-exports the macros:

```rust
use plugin_system::{plugin_export, plugin_interface, define_plugin};
```

## Macros

### `#[plugin_export]`

Attribute macro applied to `impl Plugin for YourType` blocks. Generates the minimum required FFI exports.

```rust
use plugin_system::{Plugin, PluginMetadata, PluginContext, plugin_metadata, plugin_export};

pub struct MyPlugin { /* fields */ }

impl MyPlugin {
    pub fn new() -> Self { MyPlugin { /* ... */ } }
}

#[plugin_export]
impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_metadata! {
            name: "my-plugin",
            version: "0.1.0",
            authors: ["Author"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &PluginContext) {}

    fn on_unload(&mut self) {}

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
```

**Generated FFI symbols:**

| Symbol | Signature | Description |
|--------|-----------|-------------|
| `plugin_create` | `extern "C" fn() -> *mut ()` | Instantiates `MyPlugin::new()`, returns a boxed trait object as a raw pointer |
| `plugin_destroy` | `extern "C" fn(*mut ())` | Reconstitutes the box and drops the plugin |
| `plugin_metadata_json` | `extern "C" fn() -> *mut c_char` | Serializes `metadata()` to JSON, returns as a C string |
| `plugin_free_string` | `extern "C" fn(*mut c_char)` | Frees a C string allocated by `plugin_metadata_json` |

**Constraints:**
- Must be applied to a trait impl block
- The trait path must end in `Plugin`
- The struct must have a `pub fn new() -> Self` constructor

---

### `#[plugin_export_all]`

Extended version of `#[plugin_export]`. Generates the same base FFI exports **plus** individual FFI wrappers for every method in the impl block (except `metadata`, `on_load`, `on_unload`, `plugin_type_name`).

```rust
use plugin_system::{Plugin, PluginMetadata, PluginContext, plugin_metadata};
use plugin_system::plugin_export_all;

pub struct MyPlugin { count: u32 }

impl MyPlugin {
    pub fn new() -> Self { MyPlugin { count: 0 } }
}

#[plugin_export_all(interfaces = [TimerInterface])]
impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_metadata! {
            name: "my-plugin", version: "0.1.0",
            authors: [], dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &PluginContext) {}
    fn on_unload(&mut self) {}
    fn plugin_type_name(&self) -> &'static str { std::any::type_name::<Self>() }

    fn tick(&mut self) -> u64 {
        self.count += 1;
        self.count as u64
    }

    fn get_name(&self) -> String {
        "my-plugin".to_string()
    }
}
```

**Generated method wrappers:**

For each method (e.g., `tick`, `get_name`), generates:

```rust
// For fn tick(&mut self) -> u64
#[no_mangle]
pub extern "C" fn plugin_method_tick(
    __raw: *mut std::ffi::c_void,
) -> u64 {
    unsafe {
        let __plugin = __raw as *mut MyPlugin;
        (*__plugin).tick()
    }
}

// For fn get_name(&self) -> String
#[no_mangle]
pub extern "C" fn plugin_method_get_name(
    __raw: *const std::ffi::c_void,
) -> *mut std::ffi::c_char {
    unsafe {
        let __plugin = __raw as *const MyPlugin;
        let __result = (*__plugin).get_name();
        std::ffi::CString::new(__result).unwrap().into_raw()
    }
}
```

**Argument/return type mapping:**

| Rust type | FFI type | Conversion |
|-----------|----------|------------|
| `&self` | `*const c_void` | cast pointer |
| `&mut self` | `*mut c_void` | cast pointer |
| `String` param | `*const c_char` | `CStr::from_ptr` -> `to_str` -> `to_string` |
| `String` return | `*mut c_char` | `CString::new` -> `into_raw` |
| `&str` return | `*const c_char` | `CString::new` -> `into_raw` |
| `u8`/`u16`/`u32`/`u64`/`i8`/`i16`/`i32`/`i64`/`f32`/`f64`/`bool` | same | pass-through |
| other types | `*const c_void` | cast pointer |

**Syntax:**

```rust
#[plugin_export_all(interfaces = [TraitPath1, TraitPath2])]
impl Plugin for MyType { ... }
```

The `interfaces` list is reserved for future use (interface symbol generation).

---

### `define_plugin!`

Function-like macro that generates only the base FFI exports (`plugin_create`, `plugin_destroy`, `plugin_metadata_json`, `plugin_free_string`) without wrapping an impl block.

Use this when you define `impl Plugin for YourType` separately and only need the FFI boilerplate.

```rust
use plugin_system::{Plugin, PluginMetadata, PluginContext, plugin_metadata, define_plugin};

pub struct MyPlugin { /* ... */ }

impl MyPlugin {
    pub fn new() -> Self { MyPlugin { /* ... */ } }
}

impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_metadata! {
            name: "my-plugin", version: "0.1.0",
            authors: [], dependencies: []
        }
    }
    fn on_load(&mut self, _ctx: &PluginContext) {}
    fn on_unload(&mut self) {}
    fn plugin_type_name(&self) -> &'static str { std::any::type_name::<Self>() }
}

// Generate FFI exports separately
define_plugin!(MyPlugin);
```

**Generated symbols:** Same as `#[plugin_export]` -- `plugin_create`, `plugin_destroy`, `plugin_metadata_json`, `plugin_free_string`.

---

### `#[plugin_interface]`

## Comparison

| Macro | Applies to | Generates | Use case |
|-------|-----------|-----------|----------|
| `#[plugin_export]` | `impl Plugin for T` | 4 FFI symbols | Standard plugins |
| `#[plugin_export_all(interfaces = [...])]` | `impl Plugin for T` | 4 symbols + per-method FFI wrappers | Plugins exposing methods to FFI callers |
| `define_plugin!(T)` | type path | 4 FFI symbols | Separate impl + FFI generation |

## FFI Convention

All generated functions use the C calling convention (`extern "C"`) and are marked `#[no_mangle]`. The host application uses `libloading` to look up these symbols by name:

```rust
// What PluginManager does internally:
let create: extern "C" fn() -> *mut () = unsafe {
    *lib.get(b"plugin_create")?
};
let instance = unsafe { create() };
```

## Error Handling

All macros emit compile-time errors for:
- Applying `#[plugin_export]` or `#[plugin_export_all]` to non-trait impl blocks
- Applying to impl blocks where the trait path doesn't end in `Plugin`
- Invalid `interfaces` syntax in `#[plugin_export_all]`
