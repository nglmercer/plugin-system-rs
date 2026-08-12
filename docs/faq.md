# FAQ

## Plugin doesn't load?

**Q: I built the plugin but it doesn't appear in the plugin list.**

A: Build and stage it, which puts the `.wasm` and its manifest in `plugins/`:

```bash
sd-plugins build --release
```

Then restart the server or call `POST /api/plugins/reload`.

Native `.so` / `.dll` / `.dylib` plugins are no longer loadable at all — the
loader only accepts `.wasm`. A leftover shared library in `plugins/` is
ignored silently.

## OBS connection fails?

**Q: The OBS widget shows "Disconnected" even after clicking Connect.**

A: Check these:

1. OBS WebSocket server is enabled (Tools > WebSocket Server Settings)
2. The port matches (default: 4455)
3. If you set a password, make sure it's correct in the widget settings
4. OBS is running
5. No firewall blocking the connection

## Per-app volume not working on macOS?

**Q: The App Volume widget shows "Not supported".**

A: macOS doesn't expose per-app volume control through public APIs, so the
host's `audio` capability reports `per_app: false` there and the plugin renders
the control as unavailable. Per-app volume works on Linux (PulseAudio/PipeWire)
and Windows (COM).

## Port conflicts?

**Q: Port 3000 is already in use.**

A: Set a different port in `data/config.json`, or delete the file entirely to
let the server bind an ephemeral port (port `0` — the OS picks an available
one, and the real address is printed at startup). For a one-off override use
`SD_CORE_BIND_ADDR`:

```bash
SD_CORE_BIND_ADDR=0.0.0.0:3001 cargo run --bin sd-core
```

## WebSocket not connecting?

**Q: The web UI shows "WebSocket disconnected".**

A:

1. Make sure the backend is running on port 3000
2. Check the browser console for errors
3. In development mode, ensure the Vite proxy is configured correctly
4. Try accessing `ws://localhost:3000/ws` directly

## How to build for other platforms?

**Q: Can I cross-compile for Windows from Linux?**

A: Yes, using `cargo-zigbuild`:

```bash
# Install cargo-zigbuild
cargo install cargo-zigbuild

# Build the host for Windows x64
cargo zigbuild --release --target x86_64-pc-windows-gnu -p sd-core
```

Plugins need no cross-compilation: one `.wasm` runs on every platform. Only
the host binary has a target matrix now.

## How to add custom widgets?

**Q: I want to create a custom widget for my plugin.**

A: See [`plugin-development.md`](plugin-development.md) — the widget
integration checklist plus the full plugin guide.

## OBS widgets show "OBS plugin not available"?

**Q: The OBS widgets show an error even though OBS is connected.**

A: The OBS plugin needs to be loaded and granted the `websocket` capability.
Check:

1. `plugin_obs_wasm.wasm` and `plugin_obs_wasm.manifest.json` both exist in
   `plugins/` — the manifest is what carries the grant
2. The plugin appears in `GET /api/plugins`
3. Restart the server after staging the plugin

If the widget reports a capability error rather than a connection error, the
manifest is missing or does not list `websocket`.

## Hotkey recording doesn't work?

**Q: Clicking "Record" doesn't capture my keypress.**

A:

1. Make sure you're pressing the keys within the recording timeout
   (the UI uses 2–3 s)
2. Some keys (like Print Screen) may not be capturable
3. On Linux, you may need to grant input permissions
4. Try using the key picker instead of recording

## Volume slider doesn't update in real-time?

**Q: The volume slider shows old values.**

A: The widget polls every 2 seconds by default. You can change this in the
widget settings (Config > Refresh Interval). Lower values increase CPU usage.

## Tray icon doesn't appear?

**Q: I don't see a tray icon after starting the server.**

A: On Linux, make sure you have the required packages:

- `libappindicator-gtk3` or `libayatana-appindicator`
- `gtk3`
- `xdotool`

On some desktop environments (like Wayland), you may need a tray indicator
extension.

## How to access from mobile?

**Q: How do I control StreamDeck from my phone?**

A:

1. Start the server on your computer
2. Click the **QR** button in the web UI navigation bar
3. Scan the QR code with your phone camera
4. Or manually enter `http://<your-computer-ip>:3000` in your phone's browser

Make sure your phone and computer are on the same WiFi network. The QR code
shows the local network IP automatically.
