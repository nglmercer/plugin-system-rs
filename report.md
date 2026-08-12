# Code Audit — StreamDeck Core (Rust host + WASM plugins + web + CLI)

> **Status: all findings addressed.** See "Resolution" at the end of the
> document for what changed per finding, and for the two places where the
> audit's description of the code turned out to be inaccurate.


**Scope:** entire workspace — `crates/*` (host + plugin framework), `plugins/*` (WASM component plugins), `web/*` (Preact frontend), `crates/plugin-cli/*` (packaging/build CLI).
**Method:** manual reading of every significant module, plus `cargo check --workspace` (passes, no errors) and `cargo clippy --workspace` (results appended at the end — still running at time of writing; the workspace pulls in `wasmtime`, so clippy is slow).

---

## 1. Security — High priority

These are the most consequential findings. The app is a local control daemon that, by design, can drive OBS, inject keystrokes, and run uploaded code — yet the network surface is completely unauthenticated.

### 1.1 No authentication on the HTTP or WebSocket API
`router.rs` (`crates/sd-api/src/api/router.rs:19`) wires `CorsLayer::new().allow_origin(Any)` and `TraceLayer`, but there is **no auth middleware anywhere** in `sd-api` (`grep` for `auth|Authorization|Bearer|token` across `sd-api/src` returns nothing). Every endpoint — including plugin upload, OBS control, hotkey injection, and the SSRF proxy — is open to anyone who can reach the port.

### 1.2 Binds to `0.0.0.0` by default
`sd-core/src/main.rs:165-166` (`bind_addr`) defaults to `[0,0,0,0]:<port>`. Combined with 1.1, the unauthenticated control plane is exposed to the whole LAN, not just localhost.

### 1.3 SSRF via `/api/proxy`
`crates/sd-api/src/api/proxy.rs:23-79` fetches **any** caller-supplied URL server-side with no scheme/host allowlist. An attacker can reach internal services — cloud metadata (`http://169.254.169.254/...`), localhost admin ports, internal K8s APIs. The frontend `FetchWidget` ("proxy" mode) points straight at this (`web/src/components/FetchWidget.tsx:82-100`). This is a textbook SSRF.

### 1.4 Arbitrary plugin upload + execution with capability grants
`/api/plugins/upload` and `/update` (`crates/sd-api/src/api/plugins.rs`, `crates/sd-plugins/src/lib.rs:317 install_plugin_file` / `:354 update_plugin_file`) write a `.wasm` component and load it. `validate_uploaded_filename` only checks the extension and path separators (`sd-plugins/src/lib.rs:639`), and the sidecar manifest shipped inside is fully trusted. A plugin may request the `input` capability, which (per `crates/plugin-system/src/capabilities.rs:103-117`) grants global keystroke injection and a global keylogger view. With no auth, this is remote sandbox escape into the host desktop. At minimum: signatures/allowlisting and a clearly warned capability prompt are needed before loading any uploaded plugin.

### 1.5 CORS `allow_origin(Any)`
`router.rs:20-23` allows any origin. Harmless on its own (no credentials), but it removes any browser-side guard and pairs badly with 1.1–1.4.

---

## 2. Behavioral / correctness bugs

### 2.1 The "timer" plugin does not time anything
`plugins/plugin-timer-wasm/src/lib.rs:27-90` stores `name → seconds` in a `thread_local!` `HashMap` and `start` merely inserts and returns `{"ok": true, ...}`. Nothing ever counts down, fires, or emits an `ActionExecuted`/timer event when a duration elapses. The component is single-threaded (no host timer capability / no WASI clock callback), and the port dropped the firing behavior with no replacement. As shipped it is a key/value store wearing a timer's name — a behavioral regression versus the native predecessor and a trap for anyone wiring automation to it.

### 2.2 Timer plugin has no UI and no `WidgetType`
`web/src/lib/types.ts` `WidgetType` omits `"timer"`, and `WidgetContent.tsx:15-41` has no timer case. Of the five shipped plugins (volume, obs, system-monitor, key-simulator, timer), only timer is invisible in the dashboard. Either the plugin or its widget is missing — they are out of sync.

### 2.3 Dual manifest files that can silently diverge
Two independent sources of truth exist for every plugin's capability grants:
- `plugins/<name>.manifest.json` (committed at repo root, **the file the loader actually reads** via `detect_manifest` in `crates/plugin-system/src/manager.rs:532`, which builds `<stem>.manifest.json` next to the `.wasm`), and
- `plugins/<dir>/plugin.manifest.json` (the source file the CLI stages into the root one in `crates/plugin-cli/src/main.rs:980 build_one_plugin`).

They happen to match today, but nothing enforces it; editing the source manifest without rebuilding leaves the committed root manifest stale, and the loader will keep granting the old capabilities. Pick one location (the source `plugin.manifest.json`) and have the loader read it, or generate the root one at build time and never commit it.

### 2.4 Synchronous, potentially long host I/O inside async handlers
OBS is a WASM component (single-threaded) and its `websocket` capability is synchronous, so `handle_command` for `connect`/`get_status` performs a full network handshake + N request/response round-trips inline (`plugins/plugin-obs-wasm/src/lib.rs:144 connect`, `:200 request`, `:519 get_status`). The API handlers call this through `with_plugin_mut` → `handle_command` while holding the plugin's `RwLock` write and (in `obs.rs`/`volume.rs`/`system.rs`) a `PluginManager` read lock. A 15 s OBS connect timeout therefore blocks the tokio worker for up to 15 s and serializes every other call to that plugin. At minimum, the OBS connect should not run inside a request-to-plugin that also blocks the registry path; ideally long network work belongs behind a host task, not a synchronous capability call.

---

## 3. Robustness / concurrency

### 3.1 `EventBus::run()` is a single point of failure for all events (incl. WebSocket)
`sd-core/src/main.rs:118-121` spawns `EventBus::run()` once. That loop is the **only** dispatcher for `subscribe_all` callbacks — including the WebSocket bridge registered in `crates/sd-api/src/api/websocket.rs:31`. If `run()`'s task panics (it calls each subscriber callback directly with `cb(&event)`, `sd-events/src/lib.rs:105`) or the broadcast channel closes, **all** event delivery to WS clients stops silently. There is no supervision/restart. Make the loop resilient (catch panics per-callback, restart on close) or move dispatch off the critical path.

### 3.2 `websocket.rs` event bridge uses `tx.try_send`
`crates/sd-api/src/api/websocket.rs:32` drops events under backpressure (channel cap 100). Acceptable, but worth a deliberate note — high-frequency button events can be lost without the client knowing.

### 3.3 Lock ordering is fragile but currently safe
`manager.rs` `unload_plugin` takes `registry.read()` → `plugin.write()` → `registry.write()` in separate scopes (the read guard is dropped before the write), and `with_plugin_mut` takes `registry.read()` → `plugin.write()`. Ordering is consistent, so no deadlock today. However this is easy to break: acquiring `registry.write()` while holding `plugin.write()` (the natural shape of "mutate a plugin then re-register it") would deadlock against a request handler holding the read. A future edit could reintroduce this; consider a single ordered lock or a `parking_lot` deadlock-detection lock in debug builds.

---

## 4. Inconsistencies / maintainability

### 4.1 Three different plugin-name derivation functions
`sd-plugins/src/lib.rs:623 derive_plugin_name`, `crates/plugin-system/src/manager.rs:549 plugin_stem`, and the manifest `name` field each derive/define the plugin name with different rules (prefix stripping, `-`→`_`). E.g. file `plugin_volume_master_wasm.wasm` → derive yields `volume_master_wasm`, but the manifest `name` is `volume-master`. It works only because load paths prefer the manifest metadata; the file-stem names are then used for enable/disable state matching. Fragile and confusing — collapse to one canonical name source.

### 4.2 Committed, stale `.wasm` artifacts in `plugins/`
Built `plugin_*_wasm.wasm` files are committed at the repo root. `cmd_check` (`plugin-cli/src/main.rs:659`) only validates structure (Cargo.toml / src / lib.rs), **not** that the committed artifact matches current source. A developer who edits plugin source and forgets `sd-plugins build` ships a stale binary with no warning. Prefer building in CI and not committing artifacts, or add a checksum/check.

### 4.3 `package --build --all-platforms` won't build the WASM plugins
`cmd_package` → `build_for_target` (`plugin-cli/src/main.rs:638`) runs `cargo build --release --target <triple>` (the **host** workspace) and never builds the plugin crates (they are deliberately outside the workspace, see `discover_plugins`). For a foreign target this also tries to cross-compile `wasmtime`-based `sd-core` without a cross toolchain and will fail. The WASM plugins are platform-independent and should be built once via `sd-plugins build`, not per-target. The `--build` flag is effectively broken for cross-platform packaging.

### 4.4 `metadata_from_path` errors loudly for manifest-less wasm
`crates/plugin-system/src/manager.rs:370 metadata_from_path` returns `Err` when there is no sidecar manifest, but `list_plugin_statuses` / `plugin_status_from_manager` call it with `.ok()` and fall back to a derived name. The error is raised and then swallowed, producing noisy logs for any plugin staged without a root `<name>.manifest.json`. Either make the loader path the single authority or downgrade the log.

### 4.5 `ObsStatusResponse` vs OBS plugin `get_status` field drift is unenforced
The plugin returns a bare `ObsData` object (with extra `scenes`/`last_error` fields) and `sd-api` deserializes it into `ObsStatusResponse` (`crates/sd-api/src/api/obs.rs:99`). Serde ignores unknown fields (no `deny_unknown_fields`), so a renamed field on either side fails silently (widget shows zeros) rather than erroring. Consider `deny_unknown_fields` on the response structs or a shared schema test.

### 4.6 Minor web error-handling inconsistency
`fetchObsStatus` (`web/src/lib/api.ts:267-271`) returns `data.data` **without** checking `success`, unlike the `obsGet` helper used by every other OBS call. A transient failure therefore surfaces as "OBS plugin not loaded" (`ObsWidget.tsx:43-47`) instead of the real error. Align it with `obsGet`.

### 4.7 Dead fallback in `parse_volume_data`
`crates/sd-api/src/api/volume.rs:118-127` reads `platform_supported`/`per_app_supported` from both the top-level object **and** the nested `state`; the plugin only ever emits them at top level, so the nested branch is dead and could mask a regression if the plugin's shape changed. Simplify to the one source.

---

## 5. Per-area notes

### Host framework (`crates/plugin-system`)
- Capability enforcement at call time (`wasm.rs:166 provider`) with a clear "not granted" vs "not available" distinction is well done.
- Epoch-based call timeouts (`wasm.rs:44 EPOCH_TICK`, `:670 deadline_ticks`) and the misbehaving-plugin test fixture (`plugins/plugin-misbehaving-wasm`) are a strong containment story.
- `manifest.rs` correctly rejects retired native ABIs with actionable errors (`manifest.rs:64`).
- `HostState::reset_recording` (`wasm.rs:406`) swallows the capability-not-granted case silently (returns `Ok(())`), unlike `get_support` which returns `false`. Inconsistent, though low impact.

### WASM plugins (`plugins/*`)
- Generally clean and well-commented; the host/guest capability split is coherent.
- `plugin-obs-wasm` is the most substantial and the contract-naming translations (`sceneName`→`name`, etc.) are tested.
- `plugin-timer-wasm` is the weak link (see 2.1).

### CLI / packaging (`crates/plugin-cli`)
- `stage.rs` and `format.rs` are thorough and the platform matrix is correct.
- See 4.3 for the broken cross-build path.

### Web (`web/*`)
- Preact throughout (`main.tsx` uses `preact` `render`/`h`); the `.tsx` extension is fine with the configured JSX factory. No React/Preact mix — earlier assumption corrected.
- Widget coverage is complete for the four UI-facing plugins; timer is the gap (2.2).
- `/api/proxy` usage (FetchWidget) inherits the SSRF risk from 1.3.

---

## 6. Recommended actions (priority order)
1. Add authentication (token/session) to the API + WS, or at minimum bind to `127.0.0.1` by default and document the exposure. (1.1, 1.2)
2. Add an SSRF allowlist / block private & link-local ranges in `/api/proxy`. (1.3)
3. Gate plugin upload behind explicit capability confirmation + signatures; never auto-grant `input`. (1.4)
4. Make the timer plugin actually fire (host timer capability or a WASI clock), or remove it. (2.1)
5. Unify plugin manifests to a single source of truth. (2.3, 4.3)
6. Harden `EventBus::run()` against callback panics / channel close. (3.1)
7. Fix `package --build` cross-build path. (4.3)
8. Add `deny_unknown_fields` / schema tests bridging plugin payloads and `sd-api` response structs. (4.5)

---

## 7. `cargo clippy` results

`cargo check --workspace` → **passes, no errors/warnings**.
`cargo clippy --workspace` (and a focused run over `plugin-system`, `sd-api`, `sd-plugins`, `sd-plugins-cli`, `sd-events`, `sd-devices`, `sd-actions`, `sd-profiles`, `sd-types`, `sd-paths`) → **1 lint**, no errors:

- **`new_without_default`** — `crates/sd-caps/src/audio/mod.rs:95` `pub fn new() -> Self` for `NativeAudioProvider`. Clippy suggests implementing `Default`. Low impact (the constructor is infallible and used as `Arc::new(NativeAudioProvider::new())` in `sd-caps/src/lib.rs:56`); add `#[derive(Default)]` or `impl Default` to silence.

**Takeaway:** the Rust code is in good static shape — clippy is essentially clean. The audit's substantive findings are therefore about *security posture*, *behavioral gaps*, and *maintainability/inconsistencies* (sections 1–6), not compiler-detectable defects. The WASM plugin crates were **not** compiled here (they build with `cargo component` for `wasm32-wasip2` and require that toolchain); their review is by reading only, and no WIT/interface mismatches were found against `crates/plugin-system/wit`.

---

## 8. Resolution

Every finding above has been addressed. What follows is what changed, and where
the audit's reading of the code did not survive contact with it.

### Security (1.x)

**1.1 / 1.5 — Authentication and CORS.** `crates/sd-api/src/auth.rs` adds a
bearer token, generated on first run and stored `0600` in the user data
directory (`SD_API_TOKEN` / `SD_API_TOKEN_FILE` override). Every `/api` and
`/ws` route sits behind a `route_layer`; the comparison is constant-time. The
one exception is `GET /api/auth/token`, which answers **loopback callers only**
so the locally served dashboard can bootstrap itself — anyone who can reach that
can already read the token file. The browser attaches the token through a
`fetch` interceptor (`web/src/lib/auth.ts`) that deliberately skips
cross-origin URLs, so `FetchWidget`'s direct mode never leaks the credential to
a third party; the WebSocket uses `?token=` because a browser cannot set
handshake headers. `TokenGate` prompts when the page is open on another device.
`allow_origin(Any)` is replaced by an allowlist (the vite dev servers, plus
`SD_CORS_ALLOWED_ORIGINS`).

**1.2 — Bind address.** Defaults to `127.0.0.1`. `config.host` opts in to
network exposure, an unparseable value falls back to loopback rather than
outward, and startup prints an explicit warning plus the token when bound
non-loopback. The QR code URL now carries the token so the phone flow still
works.

**1.3 — SSRF.** `/api/proxy` rejects non-http(s) schemes, resolves the host and
refuses loopback, private, link-local, CGNAT, benchmarking, documentation and
reserved ranges in both address families — including IPv4-mapped IPv6, which is
how `::ffff:127.0.0.1` walks past a naive check. Redirects are disabled at the
client (a 302 to `169.254.169.254` is the standard way around an allowlist),
`Host` and hop-by-hop headers cannot be set by the caller, and the body is
streamed against an 8 MiB cap. `SD_PROXY_ALLOW_PRIVATE=1` is the documented
escape hatch.

**1.4 — Plugin upload.** Uploads now carry their manifest, and every capability
it requests must be acknowledged by name or the upload is refused before a byte
is written (`crates/sd-plugins/src/upload.rs`). `input` is refused even when
acknowledged unless the host sets `SD_ALLOW_UPLOADED_INPUT_CAPABILITY=1` — an
acknowledgement over the API is only as strong as the API's authentication, and
this is the capability that hands over the keyboard. An optional
`plugins/allowed-plugins.json` pins installable binaries by SHA-256. The
dashboard shows the requested capabilities, with plain-language warnings, before
asking. Updates are held to the same rules, so "update" is not the way around
the prompt.

  Note: `install_plugin_file` previously called `metadata_from_path` on the
  uploaded file, which requires a sidecar manifest the upload had no way to
  provide — so uploads could not succeed at all. That is fixed as part of this.

### Behaviour (2.x)

**2.1 — The timer now times.** A timer stores an absolute deadline on the WASI
wall clock and derives remaining time on every read, so the numbers are real
rather than an echo of the argument. `poll` reports each expiry exactly once,
which is what a host needs to turn expiry into an event without firing it on
every tick. Covered by `a_timer_counts_down_and_expires` and
`poll_reports_each_expiry_exactly_once`.

**2.2 — Timer UI.** `timer` is a `WidgetType` with three variants, a widget, an
icon, wizard config and style previews, and translations. Reaching it needed a
route: `POST /api/plugins/:name/command` is the general path for plugins with no
typed endpoints, which is the gap that let a plugin ship with no way to use it.

**2.3 / 4.1 / 4.2 — One name, one manifest, and staleness detection.**
`plugin_system::naming` is now the single definition of plugin-name derivation;
`sd-plugins` and `manager.rs` both delegate to it. It also fixes the disagreement
the audit found: `plugin_volume_master_wasm` derives to `volume-master`, which is
what that plugin's manifest actually calls it. The generated `plugins/*.wasm` and
`plugins/*.manifest.json` are gitignored as the build output they are, leaving
`plugins/<name>/plugin.manifest.json` as the single tracked definition, and
`sd-plugins check` now fails when a staged manifest has drifted from its source
or the artifact is older than the code it was built from.

**2.4 — Blocking work off the runtime.** The async helpers in
`api/helpers.rs` take the shared manager rather than a guard and acquire both
locks inside `spawn_blocking`, so a 15-second OBS connect no longer parks a
tokio worker or serialises every other caller behind it. `refresh_and_read`
keeps the refresh-then-read pair in one blocking hop, which also closes the
window where another caller could interleave between them.

### Robustness (3.x)

**3.1 / 3.2 — Event delivery.** Each `EventBus::run` subscriber is invoked
inside `catch_unwind`, so one bad callback no longer silently ends delivery for
everyone; lag and channel close are logged rather than swallowed. The WebSocket
bridge switched from `subscribe_all` to a broadcast receiver, which fixes three
things at once: it no longer depends on `run()` being alive, it reports lag
instead of dropping events invisibly, and it stops leaking a callback per
connection into a map that is never pruned.

**3.3 — Lock ordering.** Documented as an invariant on `PluginManager`, naming
the shape that would break it and why `unload_plugin` is written the awkward
way.

### Consistency (4.x, 5)

**4.4** `manifest_metadata_from_path` returns `Option`, so listing paths stop
manufacturing an error for the ordinary case of a manifest-less component.
**4.5** The OBS payload contract is now pinned by tests, including one asserting
that a renamed field *fails* deserialization rather than defaulting to a
plausible zero. **4.6** `fetchObsStatus` goes through `obsGet`, and `ObsWidget`
surfaces the server's own message. **4.7** The dead nested-`state` fallback in
`parse_volume_data` is gone. **5** `HostState::reset_recording` logs the
capability refusal it has no way to return. **7** `NativeAudioProvider`
implements `Default`.

### Where the audit was wrong

Two things worth recording, since they change what the findings mean:

- **4.2's premise.** The built `.wasm` and staged manifests were never tracked
  by git — they were untracked build output all along. The drift risk in 2.3 was
  real, but it came from the files existing in a developer's working tree with
  nothing checking them, not from being committed. Hence the staleness check in
  `sd-plugins check`, which is what actually catches it.

- **4.7's direction.** The audit called the nested-`state` lookup dead and the
  top-level one live. That is correct about the plugin — but the two tests in
  `volume.rs` fed the *nested* shape, so they were exercising only the dead
  branch and would have kept passing if the live path broke. Both tests now use
  the shape `plugin-volume-master-wasm` actually serializes.

Additionally, `pluginAccept()` in the dashboard still offered `.dll,.so,.dylib`
— the native extensions the loader rejects — so the file picker filtered out the
only file the server accepts. Now `.wasm`.

### Verification

`cargo clippy --workspace --all-targets` is clean (this surfaced one further
pre-existing lint, `approx_constant` in a `sd-types` test, now fixed).
`cargo test --workspace` and the web test suite pass. The running daemon was
exercised end to end: unauthenticated requests get 401, all three token forms
work, the loopback bootstrap answers, static files stay public, the proxy
refuses metadata/loopback/`file://` targets, a timer counts down and expires
across real time, and an upload requesting `input` is refused at both the
acknowledgement and host-opt-in gates.
