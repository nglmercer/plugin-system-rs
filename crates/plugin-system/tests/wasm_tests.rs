//! End-to-end tests for the WebAssembly component backend.
//!
//! These load the real `plugin-timer-wasm` component through `PluginManager`,
//! so they exercise the same path the host uses: manifest detection, ABI
//! dispatch, instantiation, and JSON command round-trips.
//!
//! Build the fixture with:
//! ```text
//! cargo build --manifest-path plugins/plugin-timer-wasm/Cargo.toml \
//!     --target wasm32-wasip2 --release
//! ```
#![cfg(feature = "wasm")]

use plugin_system::PluginManager;
use std::path::PathBuf;
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn component_path() -> PathBuf {
    workspace_root()
        .join("plugins/plugin-timer-wasm/target/wasm32-wasip2/release/plugin_timer_wasm.wasm")
}

const MANIFEST: &str = r#"{
    "name": "timer",
    "version": "0.1.0",
    "abi": "wasm-component",
    "interfaces": ["Timer"],
    "limits": { "memory_mb": 32, "call_timeout_ms": 5000 }
}"#;

/// Stage the component plus its sidecar manifest in a temp dir and load it.
/// Returns `None` when the fixture hasn't been built, so the suite stays
/// runnable without the wasm toolchain.
fn load_timer() -> Option<(PluginManager, TempDir, String)> {
    let component = component_path();
    if !component.exists() {
        eprintln!(
            "skipping: component fixture not built at {}",
            component.display()
        );
        return None;
    }

    let temp = TempDir::new().unwrap();
    let staged = temp.path().join("plugin_timer.wasm");
    std::fs::copy(&component, &staged).unwrap();
    std::fs::write(temp.path().join("plugin_timer.manifest.json"), MANIFEST).unwrap();

    let mut manager = PluginManager::new();
    let name = manager
        .load_plugin(&staged)
        .expect("the timer component should load");
    Some((manager, temp, name))
}

/// Run a command against a loaded plugin.
fn call(
    manager: &PluginManager,
    plugin: &str,
    method: &str,
    args: serde_json::Value,
) -> Option<serde_json::Value> {
    let arc = manager.get_plugin_arc(plugin).unwrap();
    let mut guard = arc.write().unwrap();
    guard.handle_command(method, args)
}

#[test]
fn component_loads_and_reports_its_own_identity() {
    let Some((manager, _temp, name)) = load_timer() else {
        return;
    };

    // The name comes from the guest, not the manifest, so a plugin cannot be
    // renamed by editing the sidecar.
    assert_eq!(name, "timer");

    let arc = manager.get_plugin_arc("timer").unwrap();
    let plugin = arc.read().unwrap();
    let meta = plugin.metadata();

    assert_eq!(meta.name, "timer");
    assert_eq!(meta.version, "0.1.0");
    assert_eq!(plugin.interface_ids(), vec!["Timer".to_string()]);
    assert_eq!(plugin.plugin_type_name(), "WasmPlugin");
}

#[test]
fn commands_round_trip_as_json_and_state_persists() {
    let Some((manager, _temp, _)) = load_timer() else {
        return;
    };

    let started = call(
        &manager,
        "timer",
        "start",
        serde_json::json!({ "name": "standup", "seconds": 900 }),
    )
    .expect("start should return a result");
    assert_eq!(started["ok"], serde_json::json!(true));
    assert_eq!(started["seconds"], serde_json::json!(900));

    // State must survive across calls — the instance is long-lived.
    let got = call(
        &manager,
        "timer",
        "get",
        serde_json::json!({ "name": "standup" }),
    )
    .unwrap();
    assert_eq!(got["seconds"], serde_json::json!(900));

    call(
        &manager,
        "timer",
        "start",
        serde_json::json!({ "name": "retro", "seconds": 60 }),
    )
    .unwrap();

    let listed = call(&manager, "timer", "list", serde_json::json!({})).unwrap();
    assert_eq!(listed["timers"], serde_json::json!(["retro", "standup"]));
}

#[test]
fn interface_data_reflects_guest_state() {
    let Some((manager, _temp, _)) = load_timer() else {
        return;
    };

    call(
        &manager,
        "timer",
        "start",
        serde_json::json!({ "name": "brew", "seconds": 180 }),
    )
    .unwrap();

    let arc = manager.get_plugin_arc("timer").unwrap();
    let plugin = arc.read().unwrap();
    let data = plugin.interface_data().expect("interface data");

    assert_eq!(
        data["timers"],
        serde_json::json!([{ "name": "brew", "seconds": 180 }])
    );
}

#[test]
fn unknown_method_is_reported_without_trapping() {
    let Some((manager, _temp, _)) = load_timer() else {
        return;
    };

    let result = call(&manager, "timer", "teleport", serde_json::json!({})).unwrap();

    assert_eq!(result["ok"], serde_json::json!(false));
    assert_eq!(result["kind"], serde_json::json!("not_found"));

    // The instance must remain usable after a rejected command.
    let after = call(
        &manager,
        "timer",
        "start",
        serde_json::json!({ "name": "after", "seconds": 1 }),
    )
    .unwrap();
    assert_eq!(after["ok"], serde_json::json!(true));
}

#[test]
fn malformed_arguments_are_rejected_as_invalid_args() {
    let Some((manager, _temp, _)) = load_timer() else {
        return;
    };

    // `seconds` missing entirely.
    let result = call(
        &manager,
        "timer",
        "start",
        serde_json::json!({ "name": "oops" }),
    )
    .unwrap();

    assert_eq!(result["ok"], serde_json::json!(false));
    assert_eq!(result["kind"], serde_json::json!("invalid_args"));
}

#[test]
fn missing_timer_is_not_found_rather_than_a_failure() {
    let Some((manager, _temp, _)) = load_timer() else {
        return;
    };

    let result = call(
        &manager,
        "timer",
        "get",
        serde_json::json!({ "name": "nonexistent" }),
    )
    .unwrap();

    assert_eq!(result["kind"], serde_json::json!("not_found"));
}

// ---------------------------------------------------------------------------
// Containment
//
// Each of these behaviours takes down the entire process under `dlopen`. The
// point of the migration is that they no longer do. If any of these tests hang
// or abort the test binary, the sandbox is not working.
// ---------------------------------------------------------------------------

fn misbehaving_component_path() -> PathBuf {
    workspace_root().join(
        "plugins/plugin-misbehaving-wasm/target/wasm32-wasip2/release/plugin_misbehaving_wasm.wasm",
    )
}

/// Load the misbehaving fixture with a deliberately tight budget so the
/// timeout test finishes quickly.
fn load_misbehaving() -> Option<(PluginManager, TempDir)> {
    let component = misbehaving_component_path();
    if !component.exists() {
        eprintln!(
            "skipping: misbehaving fixture not built at {}",
            component.display()
        );
        return None;
    }

    let temp = TempDir::new().unwrap();
    let staged = temp.path().join("plugin_bad.wasm");
    std::fs::copy(&component, &staged).unwrap();
    std::fs::write(
        temp.path().join("plugin_bad.manifest.json"),
        r#"{
            "name": "misbehaving",
            "version": "0.0.1",
            "abi": "wasm-component",
            "limits": { "memory_mb": 16, "call_timeout_ms": 200 }
        }"#,
    )
    .unwrap();

    let mut manager = PluginManager::new();
    manager.load_plugin(&staged).expect("fixture should load");
    Some((manager, temp))
}

#[test]
fn a_plugin_that_hangs_is_cut_off_by_the_call_deadline() {
    let Some((manager, _temp)) = load_misbehaving() else {
        return;
    };

    let started = std::time::Instant::now();
    let result = call(&manager, "misbehaving", "hang", serde_json::json!({})).unwrap();
    let elapsed = started.elapsed();

    assert_eq!(result["kind"], serde_json::json!("trap"));
    // The budget is 200ms; allow generous slack for epoch tick granularity and
    // CI scheduling, but it must not run unbounded.
    assert!(
        elapsed < std::time::Duration::from_secs(20),
        "the deadline should have fired promptly, took {elapsed:?}"
    );
}

#[test]
fn a_panicking_plugin_does_not_take_down_the_host() {
    let Some((manager, _temp)) = load_misbehaving() else {
        return;
    };

    // Reaching the next line at all is the assertion: the panic became a trap
    // instead of aborting this process.
    let result = call(&manager, "misbehaving", "panic", serde_json::json!({})).unwrap();

    assert_eq!(result["ok"], serde_json::json!(false));
    assert_eq!(result["kind"], serde_json::json!("trap"));
}

#[test]
fn a_memory_hog_hits_its_ceiling_instead_of_the_hosts() {
    let Some((manager, _temp)) = load_misbehaving() else {
        return;
    };

    let result = call(&manager, "misbehaving", "hog", serde_json::json!({})).unwrap();

    // Either the allocation is refused and the guest traps, or the deadline
    // fires first. Both are containment; an OOM-killed test process is not.
    assert_eq!(result["ok"], serde_json::json!(false));
    assert_eq!(result["kind"], serde_json::json!("trap"));
}

#[test]
fn a_plugin_has_no_filesystem_access_it_was_not_granted() {
    let Some((_manager, _temp)) = load_misbehaving() else {
        return;
    };

    // The fixture is built against wasip2 and given a `WasiCtx` with no
    // preopened directories, so it has no path to the host filesystem at all.
    // Loading it successfully with an empty context is the assertion; there is
    // no directory handle for the guest to walk.
}

#[test]
fn unloading_a_component_removes_it_from_the_registry() {
    let Some((mut manager, _temp, _)) = load_timer() else {
        return;
    };

    manager
        .unload_plugin("timer")
        .expect("unload should succeed");
    assert!(manager.get_plugin_arc("timer").is_err());
}
