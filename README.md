# Plugin System

A multiplatform Rust plugin system using `libloading` for dynamic library loading. Plugins can be loaded from files or URLs, with support for hot-reloading and inter-plugin communication.

## Features

- **Dynamic Loading** - Load `.so` / `.dylib` / `.dll` plugins at runtime via `libloading`
- **Multiple Loaders** - `FileLoader`, `UrlLoader`, `MultiLoader` for flexible plugin sources
- **Hot Reload** - Unload and reload plugins without restarting the application
- **Inter-Plugin Communication** - Plugins can discover and interact with each other via the registry
- **Cross-Platform** - Works on Linux, macOS, and Windows
- **Dependency Management** - Automatic dependency checking before loading

## Project Structure

```
libloading/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── plugin-system/           # Core library
│   │   ├── src/
│   │   │   ├── lib.rs           # Public API
│   │   │   ├── traits.rs        # Plugin trait, PluginMetadata
│   │   │   ├── error.rs         # Error types
│   │   │   ├── registry.rs      # Plugin registry
│   │   │   ├── context.rs       # PluginContext for inter-plugin access
│   │   │   ├── manager.rs       # PluginManager lifecycle
│   │   │   ├── loader.rs        # FileLoader, UrlLoader, MultiLoader
│   │   │   ├── platform.rs      # Platform helpers
│   │   │   └── macros.rs        # define_plugin!, plugin_metadata!
│   │   └── tests/
│   ├── plugin-interfaces/       # Shared trait definitions
│   ├── plugin-types/            # Concrete plugin implementations
│   ├── plugin-hello/            # Example plugin (cdylib)
│   ├── plugin-greeter/          # Example plugin (cdylib)
│   └── host-app/                # Example host application
```

## Quick Start

### 1. Build the workspace

```bash
cargo build --release
```

### 2. Run the host application

```bash
./target/release/host-app
```

On first run, it will automatically build and copy example plugins to `./plugins/`.

### 3. Run with logging

```bash
RUST_LOG=info ./target/release/host-app
```

## Usage

### Host Application

```rust
use plugin_system::{PluginManager, FileLoader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = PluginManager::new();

    // Load plugins from a directory
    manager.load_plugins_from_dir("./plugins")?;

    // Or load a specific plugin using a loader
    let loader = FileLoader::new("./plugins/my_plugin.so");
    manager.load_plugin_from_loader(&loader, "my_plugin")?;

    // List loaded plugins
    for name in manager.plugin_names() {
        if let Some(meta) = manager.plugin_metadata(&name) {
            println!("{} v{}", meta.name, meta.version);
        }
    }

    // Unload a plugin
    manager.unload_plugin("my_plugin")?;

    Ok(())
}
```

### Creating a Plugin

```rust
use plugin_system::{define_plugin, plugin_metadata, Plugin, PluginMetadata, PluginContext};

struct MyPlugin {
    data: String,
}

impl MyPlugin {
    fn new() -> Self {
        Self {
            data: "Hello".to_string(),
        }
    }
}

impl Plugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_metadata! {
            name: "my_plugin",
            version: "1.0.0",
            authors: ["Author"],
            dependencies: []
        }
    }

    fn on_load(&mut self, ctx: &PluginContext) {
        println!("Plugin loaded!");
    }

    fn on_unload(&mut self) {
        println!("Plugin unloaded!");
    }
}

// Generate the FFI exports
define_plugin!(MyPlugin);
```

Add to `Cargo.toml`:

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
plugin-system = { path = "../plugin-system" }
```

### Using Loaders

#### FileLoader

```rust
use plugin_system::{PluginManager, FileLoader};

let mut manager = PluginManager::new();
let loader = FileLoader::new("./plugins/my_plugin.so");
manager.load_plugin_from_loader(&loader, "my_plugin")?;
```

#### UrlLoader (requires `url-loader` feature)

```toml
[dependencies]
plugin-system = { path = "../plugin-system", features = ["url-loader"] }
```

```rust
use plugin_system::{PluginManager, UrlLoader};

let mut manager = PluginManager::new();
let loader = UrlLoader::with_default_cache("https://example.com/plugin.so");
manager.load_plugin_from_loader(&loader, "my_plugin")?;
```

#### MultiLoader

```rust
use plugin_system::{PluginManager, MultiLoader, FileLoader};

let mut manager = PluginManager::new();
let loader = MultiLoader::new()
    .add_file("./plugins/my_plugin.so")
    .add_url("https://example.com/plugin.so", None);

manager.load_plugin_from_loader(&loader, "my_plugin")?;
```

### Platform Helpers

```rust
use plugin_system::{library_filename, library_path, copy_cargo_plugin};

// Get platform-specific filename
let name = library_filename("hello");
// Linux: "libhello.so", macOS: "libhello.dylib", Windows: "hello.dll"

// Get full path
let path = library_path("./plugins", "hello");

// Copy a cargo-built plugin
copy_cargo_plugin(&target_dir, &plugins_dir, "plugin_hello")?;
```

## Features

Enable features in your `Cargo.toml`:

```toml
[dependencies]
plugin-system = { path = "../plugin-system", features = ["url-loader"] }
```

| Feature | Description | Default |
|---------|-------------|---------|
| `file-loader` | FileLoader support | Yes |
| `url-loader` | UrlLoader for downloading plugins | No |

## Running Tests

```bash
cargo test -p plugin-system
```

## License

MIT
