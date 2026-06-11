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

### plugin-obs

OBS Studio control via WebSocket 5.x protocol.

- **Interface**: `ObsControl`
- **Connection**:
  - `host` (String): OBS WebSocket host (default: "127.0.0.1")
  - `port` (u16): OBS WebSocket port (default: 4455)
  - `password` (Option<String>): OBS WebSocket password
- **Commands**:
  - `connect` - Connect to OBS
  - `disconnect` - Disconnect from OBS
  - `refresh` - Refresh status
  - `get_status` - Get connection + stream/record state
  - `start_stream` - Start streaming
  - `stop_stream` - Stop streaming
  - `start_record` - Start recording
  - `stop_record` - Stop recording
  - `toggle_record_pause` - Toggle record pause
  - `get_scenes` - List scenes
  - `set_scene` - Switch scene (arg: `scene_name`)
  - `get_inputs` - List inputs
  - `set_input_volume` - Set input volume (args: `input_name`, `volume`)
  - `set_input_mute` - Mute/unmute input (args: `input_name`, `muted`)
  - `toggle_virtual_cam` - Toggle virtual camera
  - `save_replay` - Save replay buffer
  - `get_transitions` - List transitions
  - `set_transition` - Set active transition (arg: `name`)
  - `get_scene_items` - List scene items (arg: `scene_name`)
  - `set_scene_item_enabled` - Toggle source visibility (args: `scene_name`, `item_id`, `enabled`)
  - `get_studio_mode` - Get studio mode state
  - `set_studio_mode` - Set studio mode (arg: `enabled`)
- **Data**:
  - `connected` (bool): Whether connected to OBS
  - `host` (String): Connected host
  - `port` (u16): Connected port
  - `stream_active` (bool): Streaming status
  - `record_active` (bool): Recording status
  - `record_paused` (bool): Record pause status
  - `virtual_cam_active` (bool): Virtual camera status
  - `replay_buffer_active` (bool): Replay buffer status
  - `current_scene` (String): Current scene name
  - `studio_mode` (bool): Studio mode enabled
  - `cpu_usage` (f64): OBS CPU usage
  - `memory_usage` (f64): OBS memory usage (MB)
  - `fps` (f64): OBS FPS

## Widgets

### Volume Control Widget (`volume-master`)

Master volume slider with device name.

| Variant | Description |
|---------|-------------|
| `minimal` | Just volume % and mute button |
| `compact` | Slider with device name and mute toggle |
| `detailed` | Full controls with per-app section |

### App Volume Widget (`volume-apps`)

Per-app volume control for active audio streams.

| Variant | Description |
|---------|-------------|
| `minimal` | App count + mini list |
| `compact` | List with individual sliders |
| `detailed` | Full per-app controls with PID |

### OBS Control Widget (`obs-control`)

Main OBS control with stream/record/virtual cam toggles.

| Variant | Description |
|---------|-------------|
| `minimal` | Status dots for stream/record |
| `compact` | Current scene + toggle buttons |
| `detailed` | Full controls + stats + transitions |

**Settings**:
- `host` (string): OBS WebSocket host (default: "127.0.0.1")
- `port` (number): OBS WebSocket port (default: 4455)
- `password` (string): OBS WebSocket password
- `refreshInterval` (number): Poll interval in ms (default: 2000)

### OBS Scenes Widget (`obs-scenes`)

Scene switcher with transitions and source visibility.

| Variant | Description |
|---------|-------------|
| `minimal` | Current scene + grid buttons |
| `compact` | Scene list with active highlight |
| `detailed` | Scenes + transitions + source toggles |

### OBS Inputs Widget (`obs-inputs`)

Per-input volume and mute controls.

| Variant | Description |
|---------|-------------|
| `minimal` | Input count + mute toggles |
| `compact` | List with sliders and mute |
| `detailed` | Full input controls with kind info |

## Building

```bash
# Build all plugins
cargo build --release -p plugin-timer -p plugin-system-monitor -p plugin-key-simulator -p plugin-volume-master -p plugin-obs

# Copy to plugins directory
mkdir -p plugins
cp target/release/libplugin_*.so plugins/
```

## API Endpoints

### Volume
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/volume` | Get master volume state + apps |
| PUT | `/api/volume/master` | Set master volume |
| PUT | `/api/volume/mute` | Set master mute |
| GET | `/api/volume/apps` | List per-app volumes |
| PUT | `/api/volume/app/volume` | Set app volume |
| PUT | `/api/volume/app/mute` | Set app mute |

### OBS
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/obs/status` | Get OBS connection + stream/record state |
| POST | `/api/obs/connect` | Connect to OBS |
| POST | `/api/obs/disconnect` | Disconnect from OBS |
| POST | `/api/obs/stream/start` | Start streaming |
| POST | `/api/obs/stream/stop` | Stop streaming |
| POST | `/api/obs/record/start` | Start recording |
| POST | `/api/obs/record/stop` | Stop recording |
| POST | `/api/obs/record/pause` | Toggle record pause |
| GET | `/api/obs/scenes` | List scenes |
| POST | `/api/obs/scenes/current` | Switch scene |
| GET | `/api/obs/inputs` | List inputs |
| PUT | `/api/obs/inputs/volume` | Set input volume |
| PUT | `/api/obs/inputs/mute` | Set input mute |
| POST | `/api/obs/virtualcam/toggle` | Toggle virtual camera |
| POST | `/api/obs/replay/save` | Save replay buffer |
| GET | `/api/obs/transitions` | List transitions |
| POST | `/api/obs/transitions/current` | Set transition |
| GET | `/api/obs/scene-items` | List scene items |
| PUT | `/api/obs/scene-item/enabled` | Toggle source visibility |
| GET | `/api/obs/studio-mode` | Get studio mode state |
| POST | `/api/obs/studio-mode` | Set studio mode |

### Plugins
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/plugins` | List loaded plugins |
| POST | `/api/plugins/reload` | Reload all plugins |
| GET | `/api/plugins/:name` | Get plugin data |
| GET | `/api/system-stats` | Get system stats |

## Testing

### Unit Tests

```bash
# Run volume plugin tests
cargo test -p plugin-volume-master
```

Tests cover:
- Volume parsing from pactl output
- Sink-input parsing (single, multiple, empty)
- Edge cases (missing props, unknown apps)

### Manual Test Script

```bash
./scripts/test-volume.sh
```

Tests:
- pactl commands work
- API endpoints return valid JSON
- Volume set/mute operations

### E2E Test

1. Start server: `cargo run`
2. Open `http://localhost:3000`
3. Add Volume Control widget
4. Add App Volume widget
5. Play audio (e.g., YouTube)
6. Verify apps appear in App Volume widget
7. Test slider and mute controls

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

       fn handle_command(&mut self, method: &str, args: serde_json::Value) -> Option<serde_json::Value> {
           match method {
               "my_command" => Some(serde_json::json!({"ok": true})),
               _ => None,
           }
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
6. Add wizard config in `web/src/components/WidgetWizard.tsx`

## Platform Support

| Plugin | Linux | Windows | macOS |
|--------|-------|---------|-------|
| plugin-timer | ✓ | ✓ | ✓ |
| plugin-system-monitor | ✓ | ✗ | ✗ |
| plugin-key-simulator | ✓ | ✓ | ✓ |
| plugin-volume-master | ✓ | ✓ | ✓ |
| plugin-volume-master (per-app) | ✓ | ✓ | ✗ |
| plugin-obs | ✓ | ✓ | ✓ |

## OBS WebSocket Setup

1. Open OBS Studio
2. Go to **Tools > WebSocket Server Settings**
3. Enable the WebSocket server
4. Set a password (recommended for security)
5. Note the port (default: 4455)
6. In the web UI, add an **OBS Control** widget
7. Configure the widget with your OBS host/port/password
8. Click "Connect" in the widget

The OBS plugin uses the `obws` crate (v0.15) which implements the OBS WebSocket 5.x protocol.

## Notes

- Plugin `.so` files are loaded from the `plugins/` directory at startup
- Use `POST /api/plugins/reload` to hot-reload plugins without restarting
- Plugin data is accessible via `GET /api/plugins/:name`
- Commands are dispatched via `handle_command()` trait method
