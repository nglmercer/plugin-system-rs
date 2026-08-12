# API Reference

The HTTP API is served by `sd-core` on its bound port (from `data/config.json`,
defaulting to an ephemeral port when the file is absent). It binds `127.0.0.1`
unless `host` says otherwise. Real-time events arrive over the WebSocket at
`/ws`.

## Authentication

Every endpoint below requires the API token. Present it as any one of:

- `Authorization: Bearer <token>` (preferred)
- `X-SD-Token: <token>`
- `?token=<token>` in the query string — the only option for the WebSocket,
  since a browser cannot set headers on the handshake

The token is generated on first run and stored in the user data directory; the
path and value are printed at startup, and `SD_API_TOKEN` overrides it. A
request without a valid token gets `401` and an `ApiResponse` error body.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/auth/token` | Return the API token. The one unauthenticated endpoint, and it answers **loopback callers only** — it exists so the locally served dashboard can bootstrap itself. Remote clients get `403` and must be given the token another way (the QR code carries it). |

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
| POST | `/api/plugins/upload` | Upload a plugin. See [Uploading a plugin](#uploading-a-plugin). |
| POST | `/api/plugins/refresh` | Re-scan the plugins directory |
| POST | `/api/plugins/reload` | Reload all plugins |
| POST | `/api/plugins/:plugin_name/update` | Replace a plugin's binary. Same admission rules as upload. |
| GET | `/api/plugins/:plugin_name` | Get plugin data |
| DELETE | `/api/plugins/:plugin_name` | Uninstall a plugin |
| PUT | `/api/plugins/:plugin_name/enabled` | Enable/disable a plugin |
| POST | `/api/plugins/:plugin_name/command` | Invoke an arbitrary command on a loaded plugin (`{"method": "...", "args": {...}}`). The general path for plugins with no typed endpoints of their own, such as `timer`. |
| GET | `/api/system-stats` | Get system stats |
| GET | `/api/local-ip` | Get the local network URL (IP + port) |
| GET | `/api/icon/:name` | Resolve a freedesktop icon name (404 when unknown) |
| GET | `/api/dashboard` | Get dashboard layout |
| PUT | `/api/dashboard` | Save dashboard layout |
| POST | `/api/proxy` | Proxy an outbound HTTP request. Refuses non-http(s) schemes and any address that is loopback, private, link-local or otherwise not publicly routable; does not follow redirects; caps the response at 8 MiB. Set `SD_PROXY_ALLOW_PRIVATE=1` to allow private destinations. |
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

## Uploading a plugin

`POST /api/plugins/upload` takes a `multipart/form-data` body:

| Field | Required | Description |
|-------|----------|-------------|
| `file` | yes | The `.wasm` component |
| `manifest` | no | The plugin's `plugin.manifest.json`, as text. Without it the plugin is installed with **no host capabilities**. |
| `acknowledge_capability` | no | Repeat once per capability you agree to grant. `acknowledge_capabilities` accepts a comma-separated list instead. |
| `enabled` | no | `false`/`0` to install without loading. Defaults to enabled. |

A capability listed in the manifest but not acknowledged is refused, and nothing
is written to disk — so a client cannot install a plugin whose powers it never
showed the user. `input` is refused outright unless the host sets
`SD_ALLOW_UPLOADED_INPUT_CAPABILITY=1`: it grants both synthetic keystrokes and a
global view of what is typed, which is not a decision to make over the network.

If `plugins/allowed-plugins.json` exists, it must be a JSON array of lowercase
hex SHA-256 digests, and only binaries whose hash appears there may be
installed. Absent the file there is no allowlist.
