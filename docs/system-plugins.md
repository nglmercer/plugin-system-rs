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

Every plugin is a WASI component. The CLI builds each for `wasm32-wasip2` and
stages the `.wasm` plus its manifest into `plugins/`:

```bash
rustup target add wasm32-wasip2
cargo build -p sd-plugins-cli
./target/debug/sd-plugins build --release
```

One artifact per plugin, identical on every platform.

## API Endpoints

The complete HTTP + WebSocket API (volume, OBS, hotkey, plugins, core) is
documented in [`api-reference.md`](api-reference.md).

## Testing

The WASI plugin system has a test suite in the workspace crates:

```bash
# Plugin manager, manifest, capability and WIT tests
cargo test -p plugin-system

# End-to-end: real .wasm components driving the native backends
cargo test -p sd-caps --test end_to_end
```

`crates/sd-caps/tests/end_to_end.rs` loads built components and reaches real
hardware through a granted capability (audio, input, websocket). It skips
itself when the plugins have not been built or the host has no backend, so
run `sd-plugins build --release` first.

### Manual smoke test

1. Build the plugins and start the server: `cargo run --bin sd-core`
2. Open `http://localhost:3000`
3. Add a Volume Control widget
4. Add an App Volume widget
5. Play audio (e.g., YouTube)
6. Verify apps appear in the App Volume widget
7. Test the slider and mute controls

## Creating System Plugins

See [`plugin-development.md`](plugin-development.md) for the full guide:
WIT contract, minimal guest, capability manifest, build & stage, and widget
integration for the web UI.

## Platform Support

Plugins themselves are platform-independent — one `.wasm` runs everywhere.
What varies is the host capability each one depends on:

| Capability | Used by | Linux | Windows | macOS |
|---|---|---|---|---|
| (none) | timer | ✓ | ✓ | ✓ |
| `system-info` | system-monitor | ✓ | ✓ | ✓ |
| `audio` (master) | volume-master | ✓ | ✓ | ✓ |
| `audio` (per-app) | volume-master | ✓ | ✓ | ✗ |
| `input` | key-simulator | ✓ | ✓ | ✓ |
| `websocket` | obs | ✓ | ✓ | ✓ |

## OBS WebSocket Setup

1. Open OBS Studio
2. Go to **Tools > WebSocket Server Settings**
3. Enable the WebSocket server
4. Set a password (recommended for security)
5. Note the port (default: 4455)
6. In the web UI, add an **OBS Control** widget
7. Configure the widget with your OBS host/port/password
8. Click "Connect" in the widget

The OBS plugin implements the obs-websocket 5.x protocol itself — the identify
handshake, request correlation and all request types live in the component. The
host grants only a plain `websocket` transport, so OBS traffic is parsed inside
the sandbox rather than by the host. (`obws` and its tokio runtime are gone;
components are single-threaded and the capability is synchronous.)

## Notes

- Plugin `.wasm` files are loaded from the `plugins/` directory at startup,
  together with their `*.manifest.json` sidecars
- Native `.so` / `.dll` / `.dylib` plugins are no longer loadable and are
  ignored if present
- Use `POST /api/plugins/reload` to hot-reload plugins without restarting
- Plugin data is accessible via `GET /api/plugins/:name`
- Commands are dispatched via the WIT `handle-command` export
- A plugin that hangs, panics or leaks fails on its own: the host returns an
  error and stays up
