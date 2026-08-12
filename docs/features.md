# Features

What StreamDeck Core does, at a glance.

## System Tray

- **System tray icon** with context menu (multiplatform: Linux/Windows/macOS)
- **Open in Browser** — launch the web UI in your default browser
- **Exit** — clean shutdown from the tray

## QR Code (Web UI)

- **QR button** in the navigation bar
- **Scan to connect** — shows a QR code for mobile access
- **Copy URL** — one-click copy of the dashboard URL
- Works on any device with a browser on the same network

## Plugins

Plugins are sandboxed WebAssembly components. The ones that ship with the
project:

- **System Monitor** — CPU, memory, load, uptime monitoring
- **Volume Control** — master volume + per-app volume (Linux/Windows/macOS)
- **Key Simulator** — keyboard hotkey simulation and recording
- **Timer** — countdown timers with start/stop/pause
- **OBS Control** — OBS Studio integration via obs-websocket (stream, record,
  scenes, inputs, transitions, virtual cam, replay buffer)

## Widgets

- **System Monitor** — 3 variants (minimal/compact/detailed)
- **Clock** — 3 variants (simple/digital/detailed)
- **Volume Master** — master volume slider with mute
- **Volume Apps** — per-app volume control
- **OBS Control** — stream/record/virtual cam toggles with stats
- **OBS Scenes** — scene switcher with transitions and source visibility
- **OBS Inputs** — per-input volume and mute controls
- **Send Hotkey** — trigger keyboard shortcuts
- **Open URL** — open URLs in the default browser
- **Type Text** — type text strings
- **Fetch** — poll a JSON endpoint and render the response

See [`system-plugins.md`](system-plugins.md) for the built-in plugin and
widget catalog.

## Built-in Actions

- **HotkeyAction** — send keyboard shortcuts
- **TextAction** — type text
- **OpenUrlAction** — open URLs in the browser

## Web UI

- Virtual StreamDeck with 15 buttons
- Profile management
- Plugin browser
- Real-time event feed via WebSocket
- Mobile/tablet responsive design
- Widget wizard with live preview
