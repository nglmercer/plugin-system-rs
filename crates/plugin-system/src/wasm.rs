//! WebAssembly component plugin backend (WASI Preview 2).
//!
//! A `.wasm` plugin is loaded into its own wasmtime `Store` and exposed to the
//! rest of the host through the same [`Plugin`] trait a native plugin
//! implements, so everything above [`crate::PluginManager`] is unaware of
//! which backend a given plugin uses.
//!
//! Three things are possible here that are not possible with `dlopen`:
//! a misbehaving plugin traps instead of taking down the process, it cannot
//! touch memory or syscalls the host did not hand it, and one artifact runs on
//! every platform.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::p2::add_to_linker_sync;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::context::PluginContext;
use crate::error::{PluginError, Result};
use crate::handler::SharedCommandRegistry;
use crate::manifest::{PluginManifest, ResourceLimits};
use crate::traits::{Plugin, PluginDependency, PluginMetadata};

wasmtime::component::bindgen!({
    path: "wit",
    world: "streamdeck-plugin",
    // Host imports return `wasmtime::Result` so a host-side failure surfaces
    // as a guest trap rather than a panic in the embedder.
    imports: { default: trappable },
});

use self::streamdeck::plugin::types as wit_types;

/// How often the epoch ticker advances the clock. Call deadlines are rounded
/// up to this granularity.
const EPOCH_TICK: Duration = Duration::from_millis(10);

/// Shared wasmtime engine plus the ticker thread that makes call timeouts
/// possible. Compiling a `Component` is expensive, so the engine is shared by
/// every plugin.
pub struct WasmRuntime {
    engine: Engine,
    shutdown: Arc<AtomicBool>,
}

impl WasmRuntime {
    pub fn new() -> Result<Arc<Self>> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        // Epoch interruption is what turns "plugin loops forever" from a hung
        // server into an error return.
        config.epoch_interruption(true);

        let engine = Engine::new(&config).map_err(|e| PluginError::PluginLoad {
            name: "<wasm-runtime>".into(),
            reason: format!("failed to create wasmtime engine: {e}"),
        })?;

        let shutdown = Arc::new(AtomicBool::new(false));
        std::thread::Builder::new()
            .name("wasm-epoch-ticker".into())
            .spawn({
                let engine = engine.clone();
                let shutdown = shutdown.clone();
                move || {
                    while !shutdown.load(Ordering::Relaxed) {
                        std::thread::sleep(EPOCH_TICK);
                        engine.increment_epoch();
                    }
                }
            })
            .map_err(PluginError::Io)?;

        Ok(Arc::new(Self { engine, shutdown }))
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}

impl Drop for WasmRuntime {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

/// Per-instance host state: WASI context, resource table, resource ceilings,
/// and the capability handles this plugin was granted.
struct HostState {
    ctx: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    /// Name of the owning plugin, used to reject re-entrant self-calls and to
    /// tag log lines.
    plugin_name: String,
    /// Set once the plugin is registered, so it can call its peers. Absent
    /// during `metadata`/`on_load`, when the plugin is not yet in the
    /// registry.
    peers: Option<SharedCommandRegistry>,
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

// The `types` interface is pure data; it has no host-side methods, but the
// generated linker still requires the marker impl.
impl wit_types::Host for HostState {}

impl streamdeck::plugin::host::Host for HostState {
    fn log(&mut self, level: wit_types::LogLevel, message: String) -> wasmtime::Result<()> {
        let level = match level {
            wit_types::LogLevel::Trace => log::Level::Trace,
            wit_types::LogLevel::Debug => log::Level::Debug,
            wit_types::LogLevel::Info => log::Level::Info,
            wit_types::LogLevel::Warn => log::Level::Warn,
            wit_types::LogLevel::Error => log::Level::Error,
        };
        log::log!(level, "[plugin:{}] {}", self.plugin_name, message);
        Ok(())
    }

    fn call_plugin(
        &mut self,
        plugin: String,
        method: String,
        args_json: String,
    ) -> wasmtime::Result<std::result::Result<String, String>> {
        // A plugin calling itself would re-enter its own `Store`, which is
        // already mutably borrowed for this call. Refuse rather than deadlock.
        if plugin == self.plugin_name {
            return Ok(Err(format!(
                "plugin '{plugin}' cannot call itself re-entrantly"
            )));
        }

        let peers = match &self.peers {
            Some(p) => p,
            None => {
                return Ok(Err(
                    "the plugin registry is not available yet; call-plugin cannot be used before on-load completes".into(),
                ))
            }
        };

        let args: serde_json::Value = match serde_json::from_str(&args_json) {
            Ok(v) => v,
            Err(e) => return Ok(Err(format!("args are not valid JSON: {e}"))),
        };

        let registry = peers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match registry.handle_command(&plugin, &method, args) {
            Some(value) => Ok(Ok(value.to_string())),
            None => Ok(Err(format!("no such plugin or method: {plugin}.{method}"))),
        }
    }
}

/// A component instance and everything needed to call into it.
struct Instance {
    store: Store<HostState>,
    bindings: StreamdeckPlugin,
    /// Wall-clock budget per call, expressed in epoch ticks.
    deadline_ticks: u64,
}

impl Instance {
    /// Arm the epoch deadline before each guest call, so the budget applies
    /// per call rather than for the lifetime of the instance.
    fn arm(&mut self) {
        self.store.set_epoch_deadline(self.deadline_ticks);
    }

    /// Borrow the guest exports and the store as disjoint fields.
    ///
    /// Going through a `MutexGuard`'s `DerefMut` would borrow the whole guard,
    /// so callers must reborrow once and split here instead.
    fn split(&mut self) -> (GuestExports<'_>, &mut Store<HostState>) {
        (self.bindings.streamdeck_plugin_guest(), &mut self.store)
    }
}

type GuestExports<'a> = &'a exports::streamdeck::plugin::guest::Guest;

/// A plugin implemented as a WebAssembly component.
pub struct WasmPlugin {
    /// `Store` is `Send` but not `Sync`, and `Plugin` requires both. The mutex
    /// also enforces the one-call-at-a-time discipline the guest expects.
    inner: Mutex<Instance>,
    metadata: PluginMetadata,
    interface_ids: Vec<String>,
}

impl WasmPlugin {
    /// Compile and instantiate a component from disk.
    pub fn load(runtime: &WasmRuntime, bytes: &[u8], manifest: &PluginManifest) -> Result<Self> {
        let name = manifest.name.clone();

        let component =
            Component::new(runtime.engine(), bytes).map_err(|e| PluginError::PluginLoad {
                name: name.clone(),
                reason: format!("not a valid WebAssembly component: {e}"),
            })?;

        let mut linker: Linker<HostState> = Linker::new(runtime.engine());
        add_to_linker_sync(&mut linker).map_err(|e| PluginError::PluginLoad {
            name: name.clone(),
            reason: format!("failed to add WASI to the linker: {e}"),
        })?;
        StreamdeckPlugin::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
            &mut linker,
            |state| state,
        )
        .map_err(|e| PluginError::PluginLoad {
            name: name.clone(),
            reason: format!("failed to add host interface to the linker: {e}"),
        })?;

        let state = HostState {
            // A plugin gets no preopened directories, no environment, and no
            // inherited stdio. Anything it needs must arrive as a capability.
            ctx: WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
            limits: limits_from(&manifest.limits),
            plugin_name: name.clone(),
            peers: None,
        };

        let mut store = Store::new(runtime.engine(), state);
        store.limiter(|state| &mut state.limits);

        let deadline_ticks = deadline_ticks(&manifest.limits);
        store.set_epoch_deadline(deadline_ticks);

        let bindings =
            StreamdeckPlugin::instantiate(&mut store, &component, &linker).map_err(|e| {
                PluginError::PluginLoad {
                    name: name.clone(),
                    reason: format!("instantiation failed: {e}"),
                }
            })?;

        let mut instance = Instance {
            store,
            bindings,
            deadline_ticks,
        };

        // Identity comes from the guest, so a plugin cannot be silently
        // renamed by editing its manifest.
        instance.arm();
        let wit_meta = instance
            .bindings
            .streamdeck_plugin_guest()
            .call_get_metadata(&mut instance.store)
            .map_err(|e| trap(&name, "metadata", &e))?;

        instance.arm();
        let interface_ids = instance
            .bindings
            .streamdeck_plugin_guest()
            .call_interface_ids(&mut instance.store)
            .map_err(|e| trap(&name, "interface-ids", &e))?;

        let metadata = PluginMetadata {
            name: wit_meta.name,
            version: wit_meta.version,
            authors: wit_meta.authors,
            dependencies: wit_meta
                .dependencies
                .into_iter()
                .map(|d| PluginDependency {
                    name: d.name,
                    version_req: d.version_req,
                })
                .collect(),
        };

        if metadata.name != manifest.name {
            log::warn!(
                "plugin '{}' reports the name '{}'; using the reported name",
                manifest.name,
                metadata.name
            );
        }

        Ok(Self {
            inner: Mutex::new(instance),
            metadata,
            interface_ids,
        })
    }

    /// Hand the plugin its view of the other loaded plugins. Called once the
    /// plugin is in the registry.
    pub fn set_peers(&self, peers: SharedCommandRegistry) {
        let mut guard = self.lock();
        guard.store.data_mut().peers = Some(peers);
    }

    pub fn metadata_ref(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Instance> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn limits_from(limits: &ResourceLimits) -> StoreLimits {
    StoreLimitsBuilder::new()
        .memory_size((limits.memory_mb as usize) * 1024 * 1024)
        // Deliberately no instance cap: a single component expands to several
        // core wasm instances (the component plus its inner modules and
        // adapters), so a count here would constrain the toolchain's output
        // rather than the plugin's appetite. Memory is the meaningful limit.
        .build()
}

fn deadline_ticks(limits: &ResourceLimits) -> u64 {
    let tick_ms = EPOCH_TICK.as_millis() as u64;
    limits.call_timeout_ms.div_ceil(tick_ms).max(1)
}

fn trap(plugin: &str, method: &str, err: &wasmtime::Error) -> PluginError {
    PluginError::GuestTrap {
        name: plugin.to_string(),
        method: method.to_string(),
        reason: format!("{err:#}"),
    }
}

impl Plugin for WasmPlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata.clone()
    }

    fn on_load(&mut self, _ctx: &PluginContext) {
        let mut guard = self.lock();
        guard.arm();
        let (guest, store) = guard.split();
        if let Err(e) = guest.call_on_load(store) {
            // A trap here is contained: the plugin simply does not come up.
            log::error!(
                "plugin '{}' trapped during on-load: {e:#}",
                self.metadata.name
            );
        }
    }

    fn on_unload(&mut self) {
        let mut guard = self.lock();
        guard.arm();
        let (guest, store) = guard.split();
        if let Err(e) = guest.call_on_unload(store) {
            log::error!(
                "plugin '{}' trapped during on-unload: {e:#}",
                self.metadata.name
            );
        }
    }

    fn plugin_type_name(&self) -> &'static str {
        "WasmPlugin"
    }

    fn interface_ids(&self) -> Vec<String> {
        self.interface_ids.clone()
    }

    fn interface_data(&self) -> Option<serde_json::Value> {
        let mut guard = self.lock();
        guard.arm();
        let (guest, store) = guard.split();
        let raw = match guest.call_interface_data(store) {
            Ok(v) => v?,
            Err(e) => {
                log::error!(
                    "plugin '{}' trapped during interface-data: {e:#}",
                    self.metadata.name
                );
                return None;
            }
        };
        match serde_json::from_str(&raw) {
            Ok(v) => Some(v),
            Err(e) => {
                log::error!(
                    "plugin '{}' returned invalid JSON from interface-data: {e}",
                    self.metadata.name
                );
                None
            }
        }
    }

    fn handle_command(
        &mut self,
        method: &str,
        args: serde_json::Value,
    ) -> Option<serde_json::Value> {
        let args_json = match serde_json::to_string(&args) {
            Ok(s) => s,
            Err(e) => {
                log::error!("failed to serialize args for '{method}': {e}");
                return None;
            }
        };

        let mut guard = self.lock();
        guard.arm();
        let (guest, store) = guard.split();
        let result = guest.call_handle_command(store, method, &args_json);

        match result {
            Ok(Ok(json)) => match serde_json::from_str(&json) {
                Ok(v) => Some(v),
                Err(e) => {
                    log::error!(
                        "plugin '{}' returned invalid JSON from '{method}': {e}",
                        self.metadata.name
                    );
                    None
                }
            },
            Ok(Err(err)) => {
                let (kind, msg) = match err {
                    wit_types::CommandError::NotFound(m) => ("not_found", m),
                    wit_types::CommandError::InvalidArgs(m) => ("invalid_args", m),
                    wit_types::CommandError::Failed(m) => ("failed", m),
                };
                log::debug!(
                    "plugin '{}' rejected '{method}' ({kind}): {msg}",
                    self.metadata.name
                );
                Some(serde_json::json!({ "ok": false, "error": msg, "kind": kind }))
            }
            Err(e) => {
                // The guest trapped — a panic, a timeout, or an out-of-memory.
                // The host stays up and the caller gets an error.
                log::error!(
                    "plugin '{}' trapped during '{method}': {e:#}",
                    self.metadata.name
                );
                Some(serde_json::json!({
                    "ok": false,
                    "error": format!("plugin trapped: {e}"),
                    "kind": "trap",
                }))
            }
        }
    }
}
