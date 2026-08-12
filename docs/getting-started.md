# Getting Started

Install, run, and develop StreamDeck Core.

## Prerequisites

**Rust** (via [rustup](https://rustup.rs)):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Node.js** (18+; the CI runs 22):

```bash
# Via nvm
nvm install 20
```

**Platform dependencies** — the system tray requires GTK and libappindicator
on Linux:

| Platform | Command |
|----------|---------|
| Arch Linux | `pacman -S gtk3 xdotool libappindicator-gtk3` |
| Debian/Ubuntu | `sudo apt install libgtk-3-dev libxdo-dev libappindicator3-dev` |
| Windows | None required |
| macOS | None required |

## Download

Pre-built releases are available on the [Releases page](https://github.com/yourusername/streamdeck-core/releases):

| Platform | Architecture | Format |
|----------|--------------|--------|
| Linux | x86_64 | `.tar.gz` |
| Linux | ARM64 | `.tar.gz` |
| Windows | x86_64 | `.zip` |
| Windows | ARM64 | `.zip` |
| macOS | x86_64 (Intel) | `.tar.gz` |
| macOS | ARM64 (Apple Silicon) | `.tar.gz` |

Artifacts are named `streamdeck-core-<platform>.<format>` (e.g.
`streamdeck-core-linux-x64.tar.gz`), and every archive contains a
`platform.txt` describing which arch it was packaged for.

### Quick Install

```bash
# Download and extract
tar xzf streamdeck-core-linux-x64.tar.gz
cd streamdeck-core-linux-x64

# Run
./sd-core
```

The server binds the port from `data/config.json` (the repo ships one with
`"port": 3000`). The port is printed at startup, and the tray / QR code /
`/api/local-ip` all report the real address automatically. Open your browser
or scan the QR code from your phone.

To use a fixed port, set it in `data/config.json`:

```json
{ "port": 8080 }
```

**Delete `data/config.json` to fall back to an ephemeral port**: the server
binds port `0` and the OS assigns an available one on every start, so a busy
port never blocks startup. `SD_CORE_BIND_ADDR` (a full socket address)
overrides the config file for one-off runs:

```bash
SD_CORE_BIND_ADDR=0.0.0.0:8080 ./sd-core
```

## Build from Source

### Build everything (recommended)

```bash
# Build the CLI tool first
cargo build --release -p sd-plugins-cli

# Build CLI + plugins + web frontend + core binary
./target/release/sd-plugins build --release --with-web --with-core
```

Or build manually step by step:

```bash
cd web && npm ci && npm run build && cd ..
cargo build --release
```

### Run

```bash
cargo run --bin sd-core
```

Or use the pre-built binary:

```bash
./target/release/sd-core
```

A system tray icon will appear. Right-click it for options:

- **Open in Browser** — launch the web UI
- **Exit** — shutdown the server

## Development Mode

For plugin development with auto-rebuild and auto-restart:

```bash
# Build the CLI tool first
cargo build -p sd-plugins-cli

# Watch plugins + auto-restart sd-core
./target/debug/sd-plugins dev -- cargo run --bin sd-core

# Or with release mode
./target/debug/sd-plugins dev -r -- cargo run --release --bin sd-core
```

Or use the Make targets:

```bash
make dev CMD="cargo run --bin sd-core"
make dev-release CMD="cargo run --release --bin sd-core"
```

The `dev` command:

1. Builds all plugins once
2. Runs your command (e.g. `cargo run --bin sd-core`)
3. Watches `plugins/*/src/` and `crates/plugin-system/src/` for changes
4. On change: rebuilds affected plugins and restarts your command
5. Press `Ctrl+C` to stop

For frontend development with hot reload:

```bash
cd web
npm run dev
```

The Vite dev server starts on `http://localhost:5173` and proxies `/api` and
`/ws` to the backend on port 3000 — the port set by the repo's
`data/config.json`, so keep that file in place for frontend development.

## OBS Setup

1. Open OBS Studio
2. Go to **Tools > WebSocket Server Settings**
3. Enable the WebSocket server
4. Set a password (recommended)
5. Note the port (default: 4455)
6. In the web UI, add an **OBS Control** widget
7. Configure the widget with your OBS host/port/password
8. Click "Connect" in the widget
