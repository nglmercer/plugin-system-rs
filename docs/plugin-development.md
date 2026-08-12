# Creating a Plugin

A plugin is a `cdylib` built for `wasm32-wasip2`, implementing the WIT world in
[`crates/plugin-system/wit/plugin.wit`](../crates/plugin-system/wit/plugin.wit).
Point `wit-bindgen` at that file rather than vendoring a copy — a contract with
two definitions is a contract that drifts.

Plugin crates live in `plugins/`. They are **excluded** from the cargo
workspace (see the `exclude` list in the root `Cargo.toml`): the
wit-bindgen glue emits wasm-only imports that cannot link into a host binary.
Build them with `sd-plugins build`, or `cargo build` from inside the plugin's
directory.

## Scaffold

`Cargo.toml`:

```toml
[package]
name = "my-plugin-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wit-bindgen = "0.46"
serde_json = "1"
```

`src/lib.rs`:

```rust
wit_bindgen::generate!({
    path: "../../crates/plugin-system/wit",
    world: "streamdeck-plugin",
});

use exports::streamdeck::plugin::guest::Guest;
use streamdeck::plugin::types::{CommandError, Metadata};

struct MyPlugin;

impl Guest for MyPlugin {
    fn get_metadata() -> Metadata {
        Metadata {
            name: "my-plugin".into(),
            version: "0.1.0".into(),
            authors: vec!["You".into()],
            dependencies: vec![],
        }
    }

    fn on_load() {}
    fn on_unload() {}

    fn interface_ids() -> Vec<String> {
        vec!["MyInterface".into()]
    }

    fn interface_data() -> Option<String> {
        None
    }

    fn handle_command(method: String, args_json: String) -> Result<String, CommandError> {
        let _args: serde_json::Value = serde_json::from_str(&args_json)
            .map_err(|e| CommandError::InvalidArgs(e.to_string()))?;
        match method.as_str() {
            "my_command" => Ok(r#"{"ok":true}"#.into()),
            other => Err(CommandError::NotFound(other.into())),
        }
    }
}

export!(MyPlugin);
```

## Build and stage

```bash
rustup target add wasm32-wasip2
cargo build -p sd-plugins-cli
./target/debug/sd-plugins build --release -p my-plugin-wasm
```

The component lands in `plugins/<lib_name>.wasm` (e.g. `plugins/my_plugin_wasm.wasm`).

## Capability manifest

A sidecar `plugin.manifest.json` is optional; add one to declare capability
grants and resource limits:

```json
{
  "name": "my-plugin",
  "version": "0.1.0",
  "abi": "wasm-component",
  "capabilities": [],
  "limits": { "memory_mb": 64, "call_timeout_ms": 5000 }
}
```

Available capabilities: `system-info`, `audio`, `input`, `websocket`. See
[`architecture.md`](architecture.md) for what each grants.

Identity comes from the guest's `get-metadata`, not the manifest, so a plugin
cannot be renamed by editing its sidecar.

## Widget integration

To surface your plugin in the web UI:

1. Create `web/src/components/MyWidget.tsx`
2. Add widget type to `web/src/lib/types.ts`
3. Add widget catalog entry to `web/src/components/widgetHelpers.ts`
4. Register in `web/src/components/WidgetContent.tsx`
5. Add CSS styles to `web/src/styles/widgets.css`
6. Add wizard config in `web/src/components/WidgetWizard.tsx`

## Testing

The plugin framework and its WIT contract are tested in the workspace crates:

```bash
# Plugin manager, manifest, capability and WIT tests
cargo test -p plugin-system

# End-to-end: real components driving the native backends
cargo test -p sd-caps --test end_to_end
```

`crates/sd-caps/tests/end_to_end.rs` loads built `.wasm` components and reaches
real hardware through a granted capability. Build the plugins first
(`sd-plugins build --release`) so the components exist.

## References

- [`system-plugins.md`](system-plugins.md) — catalog of the built-in plugins
  and widgets
- [`architecture.md`](architecture.md) — the WIT world, capabilities, and
  platform support
- [`api-reference.md`](api-reference.md) — the HTTP API a widget talks to
- [`wasi-migration.md`](wasi-migration.md) — why the host moved to WASI
