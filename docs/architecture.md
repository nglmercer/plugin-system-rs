# Architecture

## Repository layout

```
streamdeck/
├── crates/
│   ├── plugin-system/      Core plugin framework (WASI component loader)
│   ├── sd-types/           Shared types (ActionId, ProfileId, etc.)
│   ├── sd-events/          Event bus for inter-plugin communication
│   ├── sd-actions/         Action trait + built-in actions
│   ├── sd-profiles/        Profile management (in-memory)
│   ├── sd-devices/         Device abstraction (virtual devices)
│   ├── sd-paths/           Runtime path resolution (plugins, web, state)
│   ├── sd-api/             axum HTTP + WebSocket server
│   ├── sd-plugins/         Plugin manager integration
│   ├── sd-core/            Main binary
│   ├── plugin-cli/         Build CLI tool (sd-plugins)
│   └── sd-caps/            Native capability providers (audio, input, …)
├── plugins/            (outside the cargo workspace — built for wasm32-wasip2)
│   ├── plugin-timer-wasm/          Timer/countdown
│   ├── plugin-system-monitor-wasm/ CPU/memory/load stats
│   ├── plugin-volume-master-wasm/  Master + per-app volume
│   ├── plugin-key-simulator-wasm/  Key simulation and hotkey recording
│   ├── plugin-obs-wasm/            OBS Studio over obs-websocket 5.x
│   └── plugin-misbehaving-wasm/    Fixture that misbehaves on demand, for the
│                                   containment tests
└── web/                    Preact web UI
```

## Plugin ABI

A plugin is a **WebAssembly component** targeting WASI Preview 2. That is the
only ABI: the native shared-library backends (a Rust trait object over
`dlopen`, and a flat C ABI) were removed, along with every use of `unsafe` in
`plugin-system`, which is now `#![forbid(unsafe_code)]`.

| ABI | Artifact | Notes |
|-----|----------|-------|
| `wasm-component` (default) | `.wasm` | Sandboxed. One artifact for every platform. |

What that buys, none of which was possible with `dlopen`:

- **A crashing plugin cannot take down the host.** A panic traps at the
  boundary and comes back as an error.
- **A hanging plugin is cut off** by a per-call epoch deadline.
- **A leaking plugin hits its own memory ceiling**, not the machine's.
- **No ambient authority.** No filesystem, environment, or network access
  beyond what the manifest was granted.
- **A stable, versioned contract.** The WIT world replaces the Rust vtable,
  which broke silently whenever the compiler or dependency versions differed
  between host and plugin.
- **One build artifact per plugin** instead of one per OS × arch.

The migration is documented in [`wasi-migration.md`](wasi-migration.md).

## Capabilities

A component has no ambient authority — no filesystem, no network, no device
access. Anything a plugin needs from the machine arrives through a **host
capability**, declared in its sidecar manifest and enforced on every call:

| Capability | What it grants |
|---|---|
| `system-info` | CPU, memory, swap, load average, process counts |
| `audio` | Master and per-application volume |
| `input` | Synthetic keystrokes, and recording a hotkey |
| `websocket` | Outbound WebSocket connections |

```json
{
  "name": "obs",
  "abi": "wasm-component",
  "capabilities": ["websocket"],
  "limits": { "memory_mb": 64, "call_timeout_ms": 15000 }
}
```

Three rules make the list mean something:

- **Undeclared is unreachable.** A plugin that omits `audio` gets an error from
  every audio call, even though the host has a working backend.
- **A misspelled capability fails at load**, not silently at first use.
- **Granting is not availability.** A host built without a backend reports the
  capability as unsupported; the plugin is told which half is missing.

Be clear about what this costs: the native code did not disappear, it changed
owner. The PulseAudio, COM, CoreAudio and `rdev` code that used to live inside
plugins now lives in `sd-caps` and is compiled into the host. What changed is
that it is host code under the project's control, rather than something loaded
out of a `.so` at runtime and trusted with the whole process — and plugins on
top of it are sandboxed, portable, and individually revocable.

`input` deserves particular care: it can type into whatever window has focus
and watch everything typed. Grant it deliberately.

## Platform Support

Every plugin ships as a single `.wasm` that runs everywhere. What varies is
the **host capability** behind it, since that is the part that touches the OS.

| Capability | Backend | Linux | Windows | macOS |
|---|---|---|---|---|
| `system-info` | `sysinfo` | ✓ | ✓ | ✓ |
| `audio` (master) | PulseAudio / COM / CoreAudio | ✓ | ✓ | ✓ |
| `audio` (per-app) | PulseAudio / COM | ✓ | ✓ | ✗ |
| `input` | `rdev` | ✓ | ✓ | ✓ |
| `websocket` | `tungstenite` | ✓ | ✓ | ✓ |

A capability the host cannot serve is reported as unsupported rather than
failing every call, so a plugin can grey out a control instead of erroring —
`audio.get-support()` exists for exactly that.

On Linux, `input` needs read access to `/dev/input/event*`:

```bash
sudo usermod -a -G input $USER && newgrp input
```

## Tech Stack

- **Backend**: Rust, tokio, axum
- **Plugin System**: WebAssembly components on wasmtime / WASI Preview 2
- **Frontend**: Preact, TypeScript, Vite
- **Communication**: REST + WebSocket
- **OBS Integration**: obs-websocket 5.x, implemented in the OBS plugin itself
  over the host's `websocket` capability (the `obws` crate and its tokio
  runtime are gone; the protocol — identify handshake, request correlation,
  all request types — lives in the component)
