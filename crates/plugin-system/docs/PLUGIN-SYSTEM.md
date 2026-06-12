# plugin-system

A dynamic plugin system for Rust. Load, manage, and interact with plugins compiled as shared libraries (`.so`, `.dylib`, `.dll`) at runtime.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `file-loader` | Yes | Load plugins from local filesystem paths |
| `url-loader` | No | Download plugins from remote URLs with caching |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Host Application                      │
│  PluginManager::load_plugins_from_dir("./plugins")      │
└──────────────────────┬──────────────────────────────────┘
                       │
           ┌───────────▼───────────┐
           │     PluginManager     │
           │  - loads .so/.dylib   │
           │  - validates deps     │
           │  - manages lifecycle  │
           └───────────┬───────────┘
                       │
           ┌───────────▼───────────┐
           │    PluginRegistry     │
           │  HashMap<name, Arc<   │
           │    RwLock<Box<dyn     │
           │    Plugin>>>          │
           └───────────┬───────────┘
                       │
       ┌───────────────┼───────────────┐
       │               │               │
   ┌───▼───┐       ┌───▼───┐       ┌───▼───┐
   │Plugin │       │Plugin │       │Plugin │
   │  A    │       │  B    │       │  C    │
   └───────┘       └───────┘       └───────┘
```

## Quick Start

### 1. Define a plugin

```rust
use plugin_system::{
    Plugin, PluginMetadata, PluginContext,
    plugin_metadata, plugin_export,
};

pub struct MyPlugin {
    count: u32,
}

impl MyPlugin {
    pub fn new() -> Self {
        MyPlugin { count: 0 }
    }
}

#[plugin_export]
impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_metadata! {
            name: "my-plugin",
            version: "1.0.0",
            authors: ["Your Name"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &PluginContext) {
        println!("MyPlugin loaded!");
    }

    fn on_unload(&mut self) {
        println!("MyPlugin unloaded!");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
```

### 2. Compile as dynamic library

In your plugin's `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
plugin-system = { path = "../plugin-system" }
```

### 3. Load plugins from host application

```rust
use plugin_system::PluginManager;

fn main() {
    let mut manager = PluginManager::new();

    // Load all plugins from a directory
    let loaded = manager.load_plugins_from_dir("./plugins").unwrap();
    println!("Loaded: {:?}", loaded);

    // Interact with a plugin
    manager.with_plugin("my-plugin", |plugin| {
        println!("Type: {}", plugin.plugin_type_name());
    }).unwrap();
}
```

## Core Types

### `Plugin` trait

The core trait that all plugins must implement. Only 4 lifecycle methods -- nothing else.

```rust
pub trait Plugin: Any + Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn on_load(&mut self, ctx: &PluginContext);
    fn on_unload(&mut self);
    fn plugin_type_name(&self) -> &'static str;
}
```

Domain-specific methods (e.g., `handle_command`, `simulate_keys`) belong on your concrete plugin struct, not on the trait. Use `downcast_ref`/`downcast_mut` to access them from the host:

```rust
manager.with_plugin_mut("my-plugin", |plugin| {
    if let Some(my) = plugin.downcast_mut::<MyPlugin>() {
        my.handle_command("tick", serde_json::json!({}));
    }
});
```

Extension methods on `dyn Plugin`:

```rust
impl dyn Plugin {
    pub fn downcast_ref<T: Plugin + 'static>(&self) -> Option<&T>;
    pub fn downcast_mut<T: Plugin + 'static>(&mut self) -> Option<&mut T>;
}
```

### `PluginMetadata`

```rust
#[repr(C)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub dependencies: Vec<PluginDependency>,
}

pub struct PluginDependency {
    pub name: String,
    pub version_req: String,  // semver requirement string
}
```

### `PluginManager`

The main entry point for managing plugin lifecycles.

```rust
let mut manager = PluginManager::new();

// Loading
manager.load_plugin("./target/debug/libmy_plugin.so")?;
manager.load_plugins_from_dir("./plugins")?;

// Querying
let names = manager.plugin_names();
let is_loaded = manager.is_loaded("my-plugin");
let meta = manager.plugin_metadata("my-plugin");

// Interaction
manager.with_plugin("my-plugin", |plugin| { /* read-only */ })?;
manager.with_plugin_mut("my-plugin", |plugin| { /* mutable */ })?;

// Unloading
manager.unload_plugin("my-plugin")?;
manager.reload_plugin("my-plugin")?;
```

### `PluginRegistry`

Thread-safe plugin storage. Shared between the manager and plugin contexts.

```rust
pub struct PluginRegistry { /* private */ }

// Lookup
registry.get::<MyPluginType>();           // by TypeId
registry.get_by_name("my-plugin");        // by name

// Iteration
registry.plugin_names();
registry.iter_plugins();

// Mutation
registry.register(plugin);
registry.unregister("my-plugin");
```

### `PluginContext`

Passed to `Plugin::on_load()`, gives plugins access to the shared registry.

```rust
fn on_load(&mut self, ctx: &PluginContext) {
    // Access other plugins
    ctx.with_plugin("other-plugin", |other| {
        // interact with other plugin
    });

    // Register a sub-plugin
    ctx.registry_mut().register(my_sub_plugin);
}
```

### `PluginLoader` trait

Abstraction for loading plugin binaries from different sources.

```rust
pub trait PluginLoader {
    fn load(&self) -> Result<Vec<u8>>;
    fn source(&self) -> String;
    fn exists(&self) -> bool;
}
```

Implementations:

| Loader | Description |
|--------|-------------|
| `FileLoader` | Load from a local file path |
| `UrlLoader` | Download from a URL with 1-hour cache (requires `url-loader` feature) |
| `MultiLoader` | Try multiple loaders in order |

```rust
use plugin_system::{FileLoader, MultiLoader, PluginManager};

// Single file
let loader = FileLoader::new("./plugins/libtimer.so");
manager.load_plugin_from_loader(&loader, "timer")?;

// Multiple sources with fallback
let loader = MultiLoader::new()
    .add_file("./plugins/libcache.so")
    .add_url("https://example.com/libcache.so", None);
manager.load_plugin_from_loader(&loader, "cache")?;
```

### `PluginInfo` / `PluginResult`

Introspection types for querying plugin details.

```rust
let info = manager.get_plugin_info("my-plugin")?;
println!("{} v{}", info.name, info.version);
println!("Dependencies: {:?}", info.dependencies);
```

## Dependency Resolution

When loading a plugin, the manager:

1. Reads `metadata.dependencies` (list of `{name, version_req}`)
2. Checks each dependency is present in the registry
3. Validates semver version requirements against already-loaded versions

```rust
// Plugin A declares dependency on Plugin B
plugin_metadata! {
    name: "plugin-a",
    version: "1.0.0",
    authors: [],
    dependencies: [
        dep("plugin-b", ">= 2.0"),
    ]
}
```

Errors:
- `MissingDependency` -- required plugin not loaded
- `VersionIncompatible` -- loaded version doesn't match requirement
- `InvalidSemverRequirement` -- malformed version string

## Manifest Files

Plugins can have a sidecar JSON manifest at `<lib-name>.manifest.json` in the same directory. This allows metadata inspection without loading the shared library.

```json
{
    "name": "my-plugin",
    "version": "1.0.0",
    "authors": ["Alice"],
    "dependencies": [
        {"name": "core", "version_req": ">= 2.0"}
    ]
}
```

The manager tries the manifest first; if absent, it calls the `plugin_metadata_json` FFI function.

## Platform Helpers

```rust
use plugin_system::{library_extension, library_filename, library_path, copy_plugin};

library_extension();                           // "so" / "dylib" / "dll"
library_filename("hello");                     // "libhello.so" / "hello.dll"
library_path("./plugins", "hello");            // "./plugins/libhello.so"
copy_plugin("./src/libhello.so", "./dist")?;   // copies the file
```

## Declarative Macros

```rust
// Build metadata struct
let meta = plugin_metadata! {
    name: "my-plugin",
    version: "1.0.0",
    authors: ["Author"],
    dependencies: [
        dep("dep-a", ">= 1.0"),
        dep("dep-b", "~2.3"),
    ]
};

// Build a single dependency
let d = dep!("my-dep", ">= 0.1");
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    LibraryLoad { path, reason },
    SymbolNotFound { symbol },
    PluginLoad { name, reason },
    PluginNotFound { name },
    UnloadFailed { name, reason },
    MissingDependency { plugin, dependency },
    VersionIncompatible { name, required, found },
    InvalidSemverRequirement { name, reason },
    OnLoadPanic { name, reason },
    Io(std::io::Error),
    DownloadFailed { url, reason },
    InvalidUrl(String),
    CacheError(String),
    HttpError { url, status },
}
```

## Complete Example

```rust
use plugin_system::{PluginManager, Plugin, PluginMetadata, PluginContext};
use plugin_system::{plugin_metadata, plugin_export};

// --- Plugin crate (compiled as cdylib) ---

pub struct TimerPlugin {
    count: u32,
}

impl TimerPlugin {
    pub fn new() -> Self {
        TimerPlugin { count: 0 }
    }

    // Domain-specific methods live on the concrete type
    pub fn tick(&mut self) -> u64 {
        self.count += 1;
        self.count as u64
    }
}

#[plugin_export]
impl Plugin for TimerPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_metadata! {
            name: "timer",
            version: "0.1.0",
            authors: ["Dev"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &PluginContext) {
        self.count = 0;
    }

    fn on_unload(&mut self) {}

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

// --- Host application ---

fn main() {
    let mut manager = PluginManager::new();
    manager.load_plugins_from_dir("./plugins").unwrap();

    // Access domain-specific methods via downcast
    let result = manager.with_plugin_mut("timer", |plugin| {
        plugin.downcast_mut::<TimerPlugin>().map(|t| t.tick())
    });
}
```
