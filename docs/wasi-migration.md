# FFI → WASI Migration Plan

Migrating the plugin ABI from `libloading` + Rust-vtable FFI to WebAssembly
components running on WASI Preview 2.

## 1. Why

The current loader transfers a `Box<Box<dyn Plugin>>` across the shared-library
boundary (`crates/plugin-macros/src/lib.rs:914`, consumed at
`crates/plugin-system/src/manager.rs:355`). This is the Rust ABI, which is
unstable: a plugin compiled with a different rustc version, a different
`plugin-system` revision, or different feature flags produces a vtable the host
will happily call and then crash on. The `c-flat` path in
`crates/plugin-system/src/cabi.rs` already exists as the stable-ABI escape
hatch, which tells us the problem is known.

What WASI buys:

| Property | Today (FFI) | WASI components |
|---|---|---|
| ABI stability | Rust vtable, breaks silently | Versioned WIT contract |
| A crashing plugin | Kills `sd-core` | Traps, host returns an error |
| Malicious plugin | Full process rights | Only granted capabilities |
| Build artifacts per plugin | 6 (OS × arch) | 1 `.wasm` |
| Hot reload | dlopen + PID temp files + symbol-prefix parsing | Drop the `Store`, instantiate again |
| Plugin languages | C ABI or Rust-exact | Rust, Go, C, JS, Python, … |

What it costs:

- **Native OS access disappears.** This is the whole difficulty of the
  migration and Section 4 is entirely about it.
- ~2–5× slower on call-heavy paths. Irrelevant here: every plugin call
  originates from an HTTP request or a 2-second widget poll.
- Host gains a wasmtime dependency (~10 MB in the binary) and a capability
  layer to maintain.

## 2. What actually has to move

The host↔plugin surface is already narrow and JSON-shaped
(`crates/plugin-system/src/traits.rs:35`):

```rust
fn metadata(&self) -> PluginMetadata;
fn on_load(&mut self, ctx: &PluginContext);
fn on_unload(&mut self);
fn handle_command(&mut self, method: &str, args: Value) -> Option<Value>;
fn interface_ids(&self) -> Vec<&'static str>;
fn interface_data(&self) -> Option<Value>;
```

Five of the six map onto WIT unchanged. Only `interface_ids` fights back —
`&'static str` cannot come out of a guest, and `CAbiPlugin` already leaks
`Box::leak` to satisfy it (`cabi.rs:175`). It becomes `Vec<String>`.

`PluginContext` (registry + command registry) does **not** cross the boundary
as a pointer; it becomes an *imported* WIT interface the guest calls back into.

### The key structural insight

`WasmPlugin` implements the existing `Plugin` trait, exactly as `CAbiPlugin`
does. Everything above `PluginManager` — `sd-api`, `sd-plugins`, the widget
layer, `helpers.rs:15` — is untouched for the entire migration. FFI and WASM
plugins coexist per-plugin, selected by the sidecar manifest. There is no
big-bang cutover and no point where the app is broken.

## 3. The WIT contract

`crates/plugin-system/wit/plugin.wit`:

```wit
package streamdeck:plugin@0.1.0;

interface types {
  record metadata {
    name: string, version: string,
    authors: list<string>,
    dependencies: list<tuple<string, string>>,
  }
  variant command-error { not-found(string), invalid-args(string), failed(string) }
}

/// Implemented by the guest.
interface guest {
  use types.{metadata, command-error};
  resource plugin {
    constructor();
    metadata: func() -> metadata;
    on-load: func();
    on-unload: func();
    handle-command: func(method: string, args-json: string)
      -> result<string, command-error>;
    interface-ids: func() -> list<string>;
    interface-data: func() -> option<string>;
  }
}

/// Implemented by the host, imported by the guest.
interface host {
  log: func(level: u8, msg: string);
  call-plugin: func(plugin: string, method: string, args-json: string)
    -> result<string, string>;
  emit-event: func(topic: string, payload-json: string);
  config-get: func(key: string) -> option<string>;
  config-set: func(key: string, value: string);
}

world streamdeck-plugin {
  import host;
  export guest;
}
```

JSON stays the payload format. Modelling each command's arguments in WIT would
be more type-safe but would force every widget, every `sd-api` handler, and
every `#[command]` macro expansion to change at once — a far larger diff for a
boundary that is JSON at both ends anyway (HTTP in, `serde_json::Value` out).
Revisit after the migration lands, per-interface.

## 4. Native capability triage — read this before committing

Four of five plugins call the OS directly. WASI cannot, so each dependency
needs a decision. This is the real scope of the project, and it should drive
whether you proceed.

| Plugin | Native dependency | Works in WASI? | Resolution |
|---|---|---|---|
| **plugin-timer** | none | ✅ | Direct port. Pilot. |
| **plugin-system-monitor** | `sysinfo` (reads `/proc`) | ❌ | Host `system-info` capability |
| **plugin-obs** | `obws` + tokio TCP | ⚠️ | Host `websocket` capability |
| **plugin-volume-master** | `libpulse` / COM / CoreAudio | ❌ | Host `audio` capability |
| **plugin-key-simulator** | `rdev` (X11 / uinput / Win32) | ❌ | Host `input` capability |

The pattern for all four: **the native code moves into the host as a WIT
capability interface; the plugin keeps its logic, state machine, and JSON
command surface.** For volume-master that means the ~600 lines of
per-platform PulseAudio/COM/CoreAudio code relocate from
`plugins/plugin-volume-master/src/` into a host crate (`sd-caps-audio`) behind:

```wit
interface audio {
  record app-volume { id: string, name: string, volume: f32, muted: bool }
  get-master: func() -> result<tuple<f32, bool>, string>;
  set-master: func(volume: f32) -> result<_, string>;
  set-master-mute: func(muted: bool) -> result<_, string>;
  list-apps: func() -> result<list<app-volume>, string>;
  set-app-volume: func(id: string, volume: f32) -> result<_, string>;
  set-app-mute: func(id: string, muted: bool) -> result<_, string>;
}
```

Be honest about what this is: the native code does not go away, it changes
owner. The plugin becomes a thin policy layer over a host capability. For
volume-master and key-simulator, where the plugin is *almost entirely* the
native call, ask whether they should remain plugins at all — a built-in
`sd-actions` module may be the more truthful design, with the plugin API
reserved for logic that genuinely benefits from sandboxing.

`plugin-obs` is the most favourable case: the plugin is real logic (scene
state, stats, reconnection) over one WebSocket. `wasi:sockets` exists in
Preview 2, but `obws` and `tokio` do not target it, so route OBS traffic
through a host `websocket` resource and keep the protocol handling guest-side,
or (simpler, less pure) move `obws` host-side behind an `obs-transport`
capability and keep the plugin as the command/state layer.

Also note: `plugin-obs` owns a `tokio::Runtime` and calls `block_on`
(`plugins/plugin-obs/src/lib.rs:44`). Components are single-threaded; the
runtime must go. Host capabilities become synchronous calls from the guest's
perspective, with the host doing the async work — which is simpler code than
what exists today.

## 5. Host runtime design

**wasmtime, component model, Preview 2** — not core modules on wasip1.
Components give resources, `result`/`variant` types, and versioned worlds;
wasip1 would mean hand-rolling pointer marshalling, i.e. re-inventing the
`c-flat` ABI we are trying to leave.

Concurrency: one `Store` per plugin instance, `Store` is not `Sync`, and
`handle_command` is called from axum handlers. Wrap each instance in a
`Mutex<Store>` — matching the `RwLock<Box<dyn Plugin>>` serialization the
registry already imposes (`registry.rs`, `context.rs:74`). Calls go through
`tokio::task::block_in_place` or a dedicated blocking pool so a slow plugin
cannot stall the axum worker.

Safety limits, none of which are possible today:

- **Epoch interruption** — a plugin looping forever is killed after N ms
  instead of hanging `sd-core`.
- **Memory limit** via `StoreLimits`, per plugin.
- **Capability grants declared in the manifest** and enforced at
  instantiation: a plugin that never declared `audio` cannot import it.
- **Traps become errors** — `handle_command` returns `Err`, the plugin is
  marked unhealthy, the server survives.

Manifest gains `"abi": "wasm-component"` alongside the existing `c-flat`
detection (`cabi.rs:49`), plus:

```json
{
  "name": "obs", "version": "0.1.0", "abi": "wasm-component",
  "capabilities": ["websocket", "config"],
  "limits": { "memory_mb": 64, "call_timeout_ms": 5000 }
}
```

## 6. Phases

Each phase ends with everything building and all tests green. Stop after any
phase and the system is coherent.

### Phase 0 — Prep (no WASI yet)
- `interface_ids() -> Vec<String>`; delete the `Box::leak` in `cabi.rs:175`.
- Extend `Manifest` with `abi` / `capabilities` / `limits`; keep `c-flat`
  detection working.
- Add integration tests at the `PluginManager` level asserting current FFI
  behaviour — these become the conformance suite both backends must pass.
- **Deliverable:** identical behaviour, boundary tightened.

### Phase 1 — Runtime + pilot
- New crate `plugin-wasm` (feature `wasm` on `plugin-system`): wasmtime,
  `WasmPlugin: Plugin`, `wit/plugin.wit`, host `log` + `call-plugin` imports.
- Port **plugin-timer** to `wasm32-wasip2`. Pure logic, no capabilities.
- `sd-plugins` discovery accepts `.wasm` next to `.so`/`.dll`/`.dylib`
  (`crates/sd-plugins/src/lib.rs:596`).
- **Deliverable:** timer runs sandboxed; FFI plugins unaffected. This is the
  proof point — if the API surface, hot reload, and widget flow work for
  timer, the rest is capability plumbing.

### Phase 2 — Macro parity
- `#[plugin_export]` gains a wasm branch emitting `wit-bindgen` glue instead
  of `#[no_mangle] extern "C"`, gated on `target_arch = "wasm32"`. Plugin
  *source* stays unchanged; only `Cargo.toml` and the build target change.
- `sd-plugins build --target wasm` in the CLI; `check` validates declared
  capabilities against the WIT world.
- **Deliverable:** a plugin author writes the same code as today.

### Phase 3 — Capabilities (the long phase)
One capability per PR, each with a native host implementation and a port:
1. `system-info` → port **plugin-system-monitor** (easiest; read-only).
2. `websocket` → port **plugin-obs** (highest value; most real logic).
3. `audio` → port **plugin-volume-master** (largest native relocation).
4. `input` → port **plugin-key-simulator** (decide first whether it stays a
   plugin at all).
- **Deliverable after each:** that plugin ships as `.wasm`, one artifact for
  all six platforms.

### Phase 4 — Packaging
- `.wasm` plugins drop out of the per-platform matrix in `release.yml` and
  `sd-plugins pkg`; ship one copy in every bundle.
- Plugins become genuinely distributable — a downloaded `.wasm` from an
  untrusted source is now a *reasonable* thing to run, which it is not today.

### Phase 5 — Retire FFI
- Only once every first-party plugin is ported and has soaked in a release.
- `libloading` path moves behind a non-default `native-ffi` feature, then is
  deleted along with `cabi.rs`, `prefix_from_path`, the PID temp-file dance
  (`manager.rs:104–255`), and the `.so` copy logic in `platform.rs`.
- Net effect: `manager.rs` loses most of its complexity, since nearly all of
  it exists to work around dlopen.

## 7. Risks

| Risk | Mitigation |
|---|---|
| Capability layer is most of the work and is invisible to users | Phase 3 is sequenced per-plugin with value delivered each step; can stop after OBS |
| Volume/key plugins become empty shells over host code | Decide in Phase 3 whether they become built-ins instead — cheaper than porting |
| `obws`/`tokio` cannot be made to work guest-side | Fall back to host-side `obs-transport`; plugin keeps state/command logic |
| Third-party FFI plugins exist in the wild | Phase 5 is gated on a deprecation release; `native-ffi` feature keeps them loadable |
| wasmtime adds binary size / build time | Measure in Phase 1; it is a hard gate on proceeding |

## 8. Recommendation

Do Phases 0–2. They are self-contained, they fix the genuine soundness bug in
the current ABI, and they produce a working sandboxed plugin end to end for
modest effort.

Then reassess Phase 3 with real numbers. The honest summary is that WASI is an
excellent fit for the *plugin protocol* and a poor fit for *these particular
plugins*, four of which exist mainly to touch hardware. The migration's value
is highest if you expect third-party or downloadable plugins — sandboxing and
single-artifact distribution are transformative there. If plugins will only
ever be the five first-party ones shipped in the same binary, Phase 0 plus
promoting `c-flat` to the only supported ABI achieves most of the stability
benefit for a fraction of the work.
