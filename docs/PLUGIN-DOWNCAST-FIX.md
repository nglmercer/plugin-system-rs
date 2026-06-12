# Plugin Downcast Fix — Proposal

## Current Problem

`sd-api` statically links plugin crates (`plugin-obs`, `plugin-volume-master`, etc.) to access their concrete types for `downcast_ref`/`downcast_mut`. But when plugins are loaded dynamically as cdylibs, **`TypeId` doesn't match** between the static and dynamic compilation units, causing downcast to fail.

### Evidence

```
GET /api/system-stats → 200 OK (downcast_ref works)
GET /api/obs/status   → "OBS plugin not available" (downcast_mut fails)
GET /api/volume       → "Volume plugin not available" (downcast_mut fails)
```

Plugins ARE loaded successfully:
```
Loaded 5 plugins: ["key-simulator", "obs", "system-monitor", "timer", "volume-master"]
```

### Root Cause

Rust's `TypeId` is computed from a hash of the type's canonical path + crate metadata. When a crate is compiled as both `lib` (for sd-api) and `cdylib` (for dynamic loading), the metadata hashes can differ, causing `downcast_ref::<ObsPlugin>()` to return `None`.

The `dyn Plugin` vtable comes from the cdylib. When the host calls `downcast_ref`, it uses the cdylib's `Any::type_id()` to get the TypeId. The host's `TypeId::of::<ObsPlugin>()` may differ from the cdylib's, causing the check to fail.

---

## Proposed Solution: `CommandHandler` Trait (No Plugin trait modification)

Add a **separate** `CommandHandler` trait in `plugin-system`. Plugins register their handler during `on_load` via `PluginContext`. sd-api queries handlers by plugin name — no downcasting needed.

### Architecture

```
Before (broken):
  sd-api → downcast_ref::<ObsPlugin>() → TypeId mismatch → None

After (fixed):
  sd-api → plugin_manager.get_handler("obs") → &dyn CommandHandler → handle_command(...)
```

### 1. New trait in `plugin-system/src/handler.rs`

```rust
pub trait CommandHandler: Send + Sync {
    fn handle_command(&mut self, method: &str, args: serde_json::Value) -> Option<serde_json::Value>;
    fn interface_ids(&self) -> Vec<&'static str> { Vec::new() }
    fn interface_data(&self) -> Option<serde_json::Value> { None }
}
```

### 2. Handler registry in `PluginManager`

```rust
pub struct PluginManager {
    registry: SharedRegistry,
    loaded: HashMap<String, LoadedPlugin>,
    handlers: HashMap<String, Arc<RwLock<Box<dyn CommandHandler>>>>,  // NEW
}
```

### 3. Registration during `on_load`

Add to `PluginContext`:
```rust
impl PluginContext {
    pub fn register_handler(&self, handler: Box<dyn CommandHandler>) {
        // stores in the handler registry
    }
}
```

Each plugin registers during `on_load`:
```rust
fn on_load(&mut self, ctx: &PluginContext) {
    ctx.register_handler(Box::new(ObsHandler::new(self)));
}
```

### 4. sd-api uses handlers instead of downcasting

```rust
// Before (broken):
if let Some(obs) = plugin.downcast_mut::<ObsPlugin>() {
    obs.handle_command("get_status", args)
}

// After (fixed):
if let Some(handler) = manager.get_handler("obs") {
    let mut h = handler.write().await;
    h.handle_command("get_status", args)
}
```

### 5. Remove static plugin dependencies from sd-api

```toml
# sd-api/Cargo.toml — REMOVE these:
plugin-obs = { path = "../../plugins/plugin-obs" }
plugin-volume-master = { path = "../../plugins/plugin-volume-master" }
plugin-system-monitor = { path = "../../plugins/plugin-system-monitor" }
plugin-key-simulator = { path = "../../plugins/plugin-key-simulator" }
```

---

## Files to modify

| File | Change |
|------|--------|
| `crates/plugin-system/src/handler.rs` | **NEW** — `CommandHandler` trait |
| `crates/plugin-system/src/lib.rs` | Add `pub mod handler` |
| `crates/plugin-system/src/manager.rs` | Add handler registry + `get_handler()` |
| `crates/plugin-system/src/context.rs` | Add `register_handler()` |
| `crates/sd-api/Cargo.toml` | Remove plugin dependencies |
| `crates/sd-api/src/api/obs.rs` | Use handler instead of downcast |
| `crates/sd-api/src/api/volume.rs` | Use handler instead of downcast |
| `crates/sd-api/src/api/plugins.rs` | Use handler instead of downcast |
| `crates/sd-api/src/api/hotkeys.rs` | Use handler instead of downcast |
| `plugins/plugin-obs/src/lib.rs` | Implement `CommandHandler` + register in `on_load` |
| `plugins/plugin-volume-master/src/lib.rs` | Implement `CommandHandler` + register in `on_load` |
| `plugins/plugin-system-monitor/src/lib.rs` | Implement `CommandHandler` + register in `on_load` |
| `plugins/plugin-key-simulator/src/lib.rs` | Implement `CommandHandler` + register in `on_load` |

## Benefits

- Plugin trait untouched (backward compatible)
- No TypeId-dependent downcasting
- Plugins are truly decoupled from sd-api
- Clean separation: Plugin = lifecycle, CommandHandler = API
- Published library stays minimal
