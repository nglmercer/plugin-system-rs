# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed — plugin ABI is now WebAssembly only (breaking)

The native FFI plugin backends are gone. A plugin is a WASI Preview 2
component; nothing else loads.

- **Removed the Rust trait-object and `c-flat` ABIs**, `libloading`, the
  `dlopen` loader, symbol-prefix parsing, and the PID-stamped temp-file dance
  that existed only to work around `dlopen`. `plugin-system` is now
  `#![forbid(unsafe_code)]` with no feature gate.
- **Removed the `plugin-macros` crate.** It existed solely to emit
  `#[no_mangle] extern "C"` glue. Guests use `wit-bindgen` against
  `crates/plugin-system/wit/plugin.wit` directly.
- **Removed the `native-ffi`, `file-loader` and `wasm` cargo features.** The
  wasm backend is unconditional; there is nothing left to gate.
- **A manifest declaring `abi: "native"` or `"c-flat"` is now a load error**
  that names the removed ABI and points at the replacement, rather than a
  silent reinterpretation. Manifests with no `abi` key default to
  `wasm-component` (previously `native`).
- Plugin discovery accepts only `.wasm`. Leftover `.so` / `.dll` / `.dylib`
  files in `plugins/` are ignored.

### Added — host capabilities

A component has no ambient authority, so anything touching the machine is a
host capability, declared per plugin and enforced on every call.

- New crate **`sd-caps`** with four providers: `system-info` (`sysinfo`),
  `audio` (PulseAudio / COM / CoreAudio), `input` (`rdev`), and `websocket`
  (`tungstenite`). This is where the platform code that used to live inside
  plugins now lives.
- `capabilities` in a plugin's sidecar manifest is now enforced: an undeclared
  capability is refused even when the host provides it, an unknown capability
  name fails at load, and a declared-but-unavailable one fails at the call with
  a message distinguishing the two cases.
- `PluginManager::set_capabilities` lets an embedder choose what to offer;
  the default is nothing.

### Changed — all five plugins ported to components

Same names, same commands, same JSON: `sd-api` and the web UI were not
modified. Each now ships as one `.wasm` for every platform instead of six
per-platform shared libraries.

- `timer`, `system-monitor`, `volume-master`, `key-simulator`, `obs`
- `obs` implements the obs-websocket 5.x protocol itself — identify handshake,
  request correlation, all request types — over a plain `websocket` grant.
  `obws` and its tokio runtime are gone.
- Plugins declare grants in a `plugin.manifest.json` beside their `Cargo.toml`,
  staged automatically by `sd-plugins build`.

### Changed — build and packaging

- `sd-plugins build` compiles plugins for `wasm32-wasip2` and stages the
  `.wasm` plus its manifest. `--target` now only affects the `sd-core` binary.
- Release bundles stage the same plugin files on every platform; plugins have
  dropped out of the per-platform artifact matrix.
- `sd-plugins list`/`clean` handle the out-of-workspace plugin crates, and skip
  crates marked `[package.metadata.sd-plugins] fixture = true`.

### Removed
- `docs/PLUGIN-DOWNCAST-FIX.md` — documented a `TypeId` mismatch between `lib`
  and `cdylib` builds, which cannot occur without dynamic loading.

### Added
- System tray icon with multiplatform support (Linux/Windows/macOS)
- QR code in web UI for mobile access
- `/api/local-ip` endpoint for network IP discovery
- OBS plugin with full WebSocket 5.x integration
- Volume master plugin with per-app control (Linux/Windows)
- Widget system with 10+ widget types
- **`sd-plugins pkg` cross-platform packaging pipeline**
  - Supports `.tar.gz`, `.zip`, `.deb` (pure Rust), `.rpm`, `.AppImage`,
    `.msi`, `.nsis`, `.dmg`, `.pkg`
  - Configured via `packaging.toml` at the repo root
  - Emits `checksums-sha256.txt` and `sbom.spdx.json` (SPDX 2.3) next to every
    artifact
  - Opt-in code signing for Windows (`signtool`), macOS (`codesign`) and
    Linux (GPG via `dpkg-sig` / `rpm --addsign`) via env vars
- `make package`, `make package-all`, `make package-platform`,
  `make package-formats` Makefile targets
- `docs/packaging.md` user guide

### Changed
- Simplified tray menu to: Status, Open in Browser, Exit
- QR code now uses real local IP from API instead of `window.location.origin`
- CI release workflow rewritten around `sd-plugins pkg`; one job per platform
  replacing the previous per-OS bash/powershell copy-paste
- Plugin staging dir moved from `releases/<v>/<p>/stage/` to
  `target/packaging/<p>/stage/` so it doesn't pollute the release output

### Fixed
- Linux event loop creation on background thread
- Menu event handling with `ControlFlow::Poll`
- QR code generation overflow panic
- `Command::new("npm")` no longer fails on Windows when the Node install
  directory isn't on `PATH` (uses `PATHEXT` resolution)

## [0.1.0] - 2026-06-11

### Added
- Initial release
- Plugin system with libloading (removed in Unreleased; see above)
- Web UI with Preact + TypeScript
- System monitor, timer, key simulator plugins
- Virtual StreamDeck device
- Profile management
- WebSocket real-time events
