# API Reference

The HTTP API is served by `sd-core` on its bound port (from `data/config.json`,
defaulting to an ephemeral port when the file is absent). Real-time events
arrive over the WebSocket at `/ws`.

## Core

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/devices` | List connected devices |
| POST | `/api/devices/:device_id/press/:button_index` | Simulate button press |
| GET | `/api/profiles` | List profiles |
| POST | `/api/profiles` | Create profile |
| GET | `/api/profiles/:profile_id` | Get profile |
| DELETE | `/api/profiles/:profile_id` | Delete profile |
| GET | `/api/actions` | List available actions |
| POST | `/api/actions` | Execute action |
| POST | `/api/actions/open-url` | Open URL in browser |
| GET | `/api/plugins` | List loaded plugins |
| POST | `/api/plugins/upload` | Upload a plugin |
| POST | `/api/plugins/refresh` | Re-scan the plugins directory |
| POST | `/api/plugins/reload` | Reload all plugins |
| POST | `/api/plugins/:plugin_name/update` | Update a plugin |
| GET | `/api/plugins/:plugin_name` | Get plugin data |
| DELETE | `/api/plugins/:plugin_name` | Uninstall a plugin |
| PUT | `/api/plugins/:plugin_name/enabled` | Enable/disable a plugin |
| GET | `/api/system-stats` | Get system stats |
| GET | `/api/local-ip` | Get the local network URL (IP + port) |
| GET | `/api/icon/:name` | Resolve a freedesktop icon name (404 when unknown) |
| GET | `/api/dashboard` | Get dashboard layout |
| PUT | `/api/dashboard` | Save dashboard layout |
| POST | `/api/proxy` | Proxy an outbound HTTP request |
| WS | `/ws` | WebSocket for real-time events |

## Volume

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/volume` | Get master volume state + apps |
| PUT | `/api/volume/master` | Set master volume |
| PUT | `/api/volume/mute` | Set master mute |
| GET | `/api/volume/apps` | List per-app volumes |
| PUT | `/api/volume/app/volume` | Set app volume |
| PUT | `/api/volume/app/mute` | Set app mute |

## OBS

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

## Hotkey

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/hotkey/send` | Send hotkey combination |
| POST | `/api/hotkey/record` | Record a hotkey (`timeout_ms` body, default 15000) |
| POST | `/api/hotkey/record/reset` | Reset hotkey recording |

The web UI passes an explicit timeout to `/api/hotkey/record` (2–3 s), and the
key-simulator plugin gives up after the timeout if no chord was pressed.
