# StreamDeck Core

A plugin-based StreamDeck control system with web UI, built in Rust.

## Architecture

```
streamdeck/
├── crates/
│   ├── plugin-system/      Core plugin framework (libloading-based)
│   ├── plugin-interfaces/  Shared trait definitions
│   ├── plugin-macros/      Proc macros for plugin exports
│   ├── sd-types/           Shared types (ActionId, ProfileId, etc.)
│   ├── sd-events/          Event bus for inter-plugin communication
│   ├── sd-actions/         Action trait + built-in actions
│   ├── sd-profiles/        Profile management (in-memory)
│   ├── sd-devices/         Device abstraction (virtual devices)
│   ├── sd-api/             axum HTTP + WebSocket server
│   ├── sd-plugins/         Plugin manager integration
│   └── sd-core/            Main binary
├── plugins/
│   ├── plugin-timer/       Timer/countdown plugin
│   ├── plugin-system-monitor/  System resource monitoring
│   └── plugin-key-simulator/  Key simulation plugin
└── web/                    Preact web UI
```

## Quick Start

### 1. Build the backend

```bash
cargo build
```

### 2. Build plugins

```bash
cargo build --release -p plugin-timer -p plugin-system-monitor -p plugin-key-simulator
```

### 3. Copy plugins

```bash
mkdir -p plugins
cp target/release/libplugin_*.so plugins/
```

### 4. Run the core

```bash
cargo run -p sd-core
```

The server starts on `http://localhost:3000`.

### 5. Run the web UI (development)

```bash
cd web
npm install
npm run dev
```

The web UI starts on `http://localhost:5173` and proxies API requests to the backend.

## Features

### Plugin System
- Dynamic loading via `libloading`
- Type-safe interface discovery
- Plugin-to-plugin communication
- Hot-reload support

### Built-in Actions
- **HotkeyAction**: Send keyboard shortcuts
- **TextAction**: Type text
- **OpenUrlAction**: Open URLs

### Example Plugins
- **Timer Plugin**: Countdown timers with start/stop
- **System Monitor**: CPU/memory usage widgets
- **Hotkey Plugin**: Custom key bindings

### Web UI
- Virtual StreamDeck with 15 buttons
- Profile management
- Plugin browser
- Real-time event feed via WebSocket
- Mobile/tablet responsive design

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/devices` | List connected devices |
| POST | `/api/devices/:id/press/:index` | Simulate button press |
| GET | `/api/profiles` | List profiles |
| POST | `/api/profiles` | Create profile |
| GET | `/api/profiles/:id` | Get profile |
| DELETE | `/api/profiles/:id` | Delete profile |
| GET | `/api/actions` | List available actions |
| GET | `/api/plugins` | List loaded plugins |
| POST | `/api/plugins/reload` | Reload all plugins |
| WS | `/ws` | WebSocket for real-time events |

## Creating a Plugin

```rust
use plugin_system::{Plugin, PluginMetadata};

pub struct MyPlugin;

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
}

#[plugin_system::plugin_export]
impl Plugin for MyPlugin {
    // ... implementation
}
```

## Tech Stack

- **Backend**: Rust, tokio, axum
- **Plugin System**: libloading, custom proc macros
- **Frontend**: Preact, TypeScript, Vite
- **Communication**: REST + WebSocket

## License

MIT
