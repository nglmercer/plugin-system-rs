# StreamDeck Core

A plugin-based StreamDeck control system with a web UI, built in Rust. Control
OBS, system volume, keyboard shortcuts, and more from a browser-based
dashboard.

- **Plugins are sandboxed WebAssembly components** (WASI Preview 2) — a
  crashing, hanging, or leaking plugin cannot take down the host.
- **One `.wasm` per plugin**, identical on every platform; only the host
  binary has a target matrix.
- **Browser dashboard** with a virtual 15-button deck, profiles, and widgets.

## Quick start

```bash
# Install Linux dependencies (GTK tray): see docs/getting-started.md

cargo build --release -p sd-plugins-cli
./target/release/sd-plugins build --release --with-web --with-core
./target/release/sd-core
```

Open `http://localhost:3000` (the port comes from `data/config.json`; delete
that file to bind an ephemeral port). A system tray icon appears — right-click
for **Open in Browser** / **Exit**.

## Features at a glance

- System tray (Linux/Windows/macOS) with "Open in Browser" and "Exit"
- QR code + copyable URL for phone access
- Plugins: system monitor, volume control, key simulator, timer, OBS
- Widgets: system monitor, clock, volume, OBS control/scenes/inputs, hotkey,
  open URL, type text
- Built-in actions: `HotkeyAction`, `TextAction`, `OpenUrlAction`
- Web UI: 15-button deck, profiles, plugin browser, live WebSocket event feed,
  responsive design, widget wizard with live preview

## Documentation

| Topic | Where |
|-------|-------|
| Getting started, download & install | [`docs/getting-started.md`](docs/getting-started.md) |
| Feature overview | [`docs/features.md`](docs/features.md) |
| Architecture, plugin ABI, capabilities | [`docs/architecture.md`](docs/architecture.md) |
| Creating a plugin | [`docs/plugin-development.md`](docs/plugin-development.md) |
| Adding a dashboard widget | [`docs/widgets.md`](docs/widgets.md) |
| Built-in plugins & widgets catalog | [`docs/system-plugins.md`](docs/system-plugins.md) |
| HTTP + WebSocket API reference | [`docs/api-reference.md`](docs/api-reference.md) |
| Packaging, releases, CI | [`docs/packaging.md`](docs/packaging.md) |
| Troubleshooting / FAQ | [`docs/faq.md`](docs/faq.md) |
| WASI migration history | [`docs/wasi-migration.md`](docs/wasi-migration.md) |

## Project layout

```
crates/       Host workspace (plugin-system, sd-api, sd-core, sd-caps, …)
plugins/      WASI components, one per plugin (built for wasm32-wasip2)
web/          Preact web UI
docs/         Documentation articles
```

## License

MIT
