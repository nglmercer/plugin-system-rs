# System Plugins

Built-in plugins that ship with StreamDeck Core.

## Plugins

### plugin-timer

Countdown timer plugin.

- **Interface**: `Timer`
- **Methods**: `start_timer`, `get_timer`, `list_timers`

### plugin-system-monitor

System resource monitoring.

- **Interface**: `SystemMonitor`
- **Data**:
  - `cpu_usage` (f64): CPU usage percentage
  - `cpu_model` (String): CPU model name
  - `cpu_cores` (usize): Number of cores
  - `memory_total` (u64): Total memory in bytes
  - `memory_used` (u64): Used memory in bytes
  - `memory_usage` (f64): Memory usage percentage
  - `swap_total` (u64): Total swap in bytes
  - `swap_used` (u64): Used swap in bytes
  - `load_avg` ([f64; 3]): 1/5/15 min load averages
  - `uptime` (u64): System uptime in seconds
  - `process_count` (usize): Number of processes
  - `thread_count` (usize): Number of threads

### plugin-key-simulator

Simulates keyboard input.

- **Interface**: `KeySimulator`
- **Methods**: `simulate_keys`, `listen_for_combo`, `reset_recording_state`

### plugin-volume-master

Multiplatform volume control.

- **Interface**: `VolumeMaster`
- **Data**:
  - `state.master_volume` (f32): Master volume (0-100)
  - `state.muted` (bool): Mute status
  - `state.default_device_name` (String): Default audio device name
  - `state.platform_supported` (bool): Whether volume control is supported
  - `state.per_app_supported` (bool): Whether per-app volume is supported
  - `apps` (Vec<AppVolume>): Per-app volumes (Linux/Windows only)

**Per-app volume** (Linux and Windows only):
  - `name` (String): Application name
  - `volume` (f32): Volume (0-100)
  - `muted` (bool): Mute status
  - `pid` (Option<u32>): Process ID

## Building

```bash
# Build all plugins
cargo build --release -p plugin-timer -p plugin-system-monitor -p plugin-key-simulator -p plugin-volume-master

# Copy to plugins directory
mkdir -p plugins
cp target/release/libplugin_*.so plugins/
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/volume` | Get master volume state |
| PUT | `/api/volume/master` | Set master volume |
| PUT | `/api/volume/mute` | Set master mute |
| GET | `/api/volume/apps` | List per-app volumes |
| PUT | `/api/volume/app/volume` | Set app volume |
| PUT | `/api/volume/app/mute` | Set app mute |
| GET | `/api/plugins` | List loaded plugins |
| POST | `/api/plugins/reload` | Reload all plugins |
| GET | `/api/plugins/:name` | Get plugin data |
| GET | `/api/system-stats` | Get system stats |

## Creating System Plugins

To create a new system plugin:

1. Create a new crate in `plugins/`:
   ```bash
   cargo init --lib plugins/plugin-name
   ```

2. Add to workspace `Cargo.toml`:
   ```toml
   members = [
       # ... existing members
       "plugins/plugin-name",
   ]
   ```

3. Set up `Cargo.toml`:
   ```toml
   [package]
   name = "plugin-name"
   version = "0.1.0"
   edition = "2021"

   [lib]
   crate-type = ["cdylib"]

   [dependencies]
   plugin-system = { path = "../../crates/plugin-system" }
   log = "0.4"
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   ```

4. Implement the plugin:
   ```rust
   use plugin_system::{Plugin, PluginMetadata};

   pub struct MyPlugin;

   #[plugin_system::plugin_export]
   impl Plugin for MyPlugin {
       fn metadata(&self) -> PluginMetadata {
           plugin_system::plugin_metadata! {
               name: "my-plugin",
               version: "0.1.0",
               authors: ["You"],
               dependencies: []
           }
       }

       fn on_load(&mut self, _ctx: &plugin_system::PluginContext) {
           log::info!("MyPlugin loaded");
       }

       fn on_unload(&mut self) {
           log::info!("MyPlugin unloading");
       }

       fn plugin_type_name(&self) -> &'static str {
           std::any::type_name::<Self>()
       }

       fn interface_ids(&self) -> Vec<&'static str> {
           vec!["MyInterface"]
       }

       fn interface_data(&self) -> Option<serde_json::Value> {
           None
       }
   }
   ```

5. Build and copy:
   ```bash
   cargo build -p plugin-name
   cp target/debug/libplugin_name.so plugins/
   ```

## Widget Integration

To add a widget for your plugin in the web UI:

1. Create `web/src/components/MyWidget.tsx`
2. Add widget type to `web/src/lib/types.ts`
3. Add widget catalog entry to `web/src/components/widgetHelpers.ts`
4. Register in `web/src/components/WidgetContent.tsx`
5. Add CSS styles to `web/src/styles/widgets.css`

## Platform Support

| Plugin | Linux | Windows | macOS |
|--------|-------|---------|-------|
| plugin-timer | ✓ | ✓ | ✓ |
| plugin-system-monitor | ✓ | ✗ | ✗ |
| plugin-key-simulator | ✓ | ✓ | ✓ |
| plugin-volume-master | ✓ | ✓ | ✓ |
| plugin-volume-master (per-app) | ✓ | ✓ | ✗ |

## Notes

- Plugin `.so` files are loaded from the `plugins/` directory at startup
- Use `POST /api/plugins/reload` to hot-reload plugins without restarting
- Plugin data is accessible via `GET /api/plugins/:name`
