//! Capability grant enforcement.
//!
//! These use a stub provider rather than the real hardware-backed ones, so
//! they assert on the *policy* — who may call what — rather than on whether
//! this particular machine has a sound card.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use plugin_system::{HostCapabilities, PluginManager, SystemInfoProvider, SystemStats};

mod common;
use common::{monitor_component, write_plugin};

/// Counts calls so a test can prove the host was or was not reached.
#[derive(Default)]
struct CountingSystemInfo {
    calls: AtomicUsize,
}

impl SystemInfoProvider for CountingSystemInfo {
    fn get_stats(&self) -> Result<SystemStats, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(SystemStats {
            cpu_usage: 42.5,
            cpu_model: "Test CPU".into(),
            cpu_cores: 8,
            memory_total: 16 * 1024 * 1024 * 1024,
            memory_used: 8 * 1024 * 1024 * 1024,
            swap_total: 0,
            swap_used: 0,
            load_avg: [1.0, 2.0, 3.0],
            uptime_seconds: 1234,
            process_count: 250,
            thread_count: 900,
        })
    }
}

fn manager_with(provider: Arc<CountingSystemInfo>) -> PluginManager {
    let mut manager = PluginManager::new();
    manager.set_capabilities(HostCapabilities::new().with_system_info(provider));
    manager
}

/// The happy path: declared in the manifest, provided by the host, call lands.
#[test]
fn a_granted_capability_reaches_the_host_provider() {
    let Some(component) = monitor_component() else {
        eprintln!("skipping: system-monitor component not built");
        return;
    };

    let temp = tempfile::tempdir().unwrap();
    let path = write_plugin(
        temp.path(),
        "plugin_system_monitor",
        &component,
        r#"{
            "name": "system-monitor", "version": "0.1.0",
            "abi": "wasm-component",
            "capabilities": ["system-info"]
        }"#,
    );

    let provider = Arc::new(CountingSystemInfo::default());
    let mut manager = manager_with(provider.clone());
    manager.load_plugin(&path).unwrap();

    // `on-load` samples once.
    assert!(
        provider.calls.load(Ordering::SeqCst) >= 1,
        "on-load should have sampled the host"
    );

    let data = manager
        .with_plugin("system-monitor", |p| p.interface_data())
        .unwrap()
        .expect("interface-data should be present");

    assert_eq!(data["cpu_usage"], 42.5);
    assert_eq!(data["cpu_model"], "Test CPU");
    assert_eq!(data["cpu_cores"], 8);
    // Derived guest-side from the host's totals.
    assert_eq!(data["memory_usage"], 50.0);
    assert_eq!(data["load_avg"][2], 3.0);
}

/// The point of the whole exercise: a plugin that did not ask for a capability
/// cannot use it, even though the host has one and the guest links against the
/// interface.
#[test]
fn an_ungranted_capability_is_refused() {
    let Some(component) = monitor_component() else {
        eprintln!("skipping: system-monitor component not built");
        return;
    };

    let temp = tempfile::tempdir().unwrap();
    let path = write_plugin(
        temp.path(),
        "plugin_system_monitor",
        &component,
        // No `capabilities` key at all.
        r#"{"name":"system-monitor","version":"0.1.0","abi":"wasm-component"}"#,
    );

    let provider = Arc::new(CountingSystemInfo::default());
    let mut manager = manager_with(provider.clone());
    manager.load_plugin(&path).unwrap();

    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        0,
        "the host provider must never be reached by an ungranted plugin"
    );

    let result = manager
        .with_plugin_mut("system-monitor", |p| {
            p.handle_command("refresh", serde_json::json!({}))
        })
        .unwrap()
        .expect("a refused capability should still return a response");

    assert_eq!(result["ok"], false);
    let error = result["error"].as_str().unwrap_or_default();
    assert!(
        error.contains("did not declare") && error.contains("system-info"),
        "error should name the missing grant, got: {error}"
    );
}

/// Granted, but this host has no such provider. The plugin still loads — a
/// host build option should not be a hard compatibility break — and the call
/// fails with a message pointing at the host rather than the manifest.
#[test]
fn a_granted_capability_the_host_lacks_fails_at_the_call() {
    let Some(component) = monitor_component() else {
        eprintln!("skipping: system-monitor component not built");
        return;
    };

    let temp = tempfile::tempdir().unwrap();
    let path = write_plugin(
        temp.path(),
        "plugin_system_monitor",
        &component,
        r#"{
            "name": "system-monitor", "version": "0.1.0",
            "abi": "wasm-component",
            "capabilities": ["system-info"]
        }"#,
    );

    // No providers registered at all.
    let mut manager = PluginManager::new();
    manager
        .load_plugin(&path)
        .expect("a plugin asking for an absent capability should still load");

    let result = manager
        .with_plugin_mut("system-monitor", |p| {
            p.handle_command("refresh", serde_json::json!({}))
        })
        .unwrap()
        .unwrap();

    assert_eq!(result["ok"], false);
    let error = result["error"].as_str().unwrap_or_default();
    assert!(
        error.contains("provides no"),
        "error should blame the host, not the manifest, got: {error}"
    );
}

/// A typo in the manifest must fail loudly at load rather than silently never
/// being granted.
#[test]
fn an_unknown_capability_name_is_rejected_at_load() {
    let Some(component) = monitor_component() else {
        eprintln!("skipping: system-monitor component not built");
        return;
    };

    let temp = tempfile::tempdir().unwrap();
    let path = write_plugin(
        temp.path(),
        "plugin_system_monitor",
        &component,
        r#"{
            "name": "system-monitor", "version": "0.1.0",
            "abi": "wasm-component",
            "capabilities": ["sysinfo"]
        }"#,
    );

    let mut manager = PluginManager::new();
    let err = manager.load_plugin(&path).unwrap_err().to_string();

    assert!(
        err.contains("unknown capability") && err.contains("sysinfo"),
        "error should name the bad capability, got: {err}"
    );
}
