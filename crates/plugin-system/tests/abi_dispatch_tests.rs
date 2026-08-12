//! Conformance tests for ABI selection at the `PluginManager` boundary.
//!
//! These pin the behaviour that must hold identically for every backend
//! (native, c-flat, wasm-component), so the WASI migration can proceed without
//! silently changing how plugins are discovered and dispatched.

use plugin_system::{Abi, PluginManager};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Write a fake plugin binary plus its sidecar manifest, returning the
/// binary's path. The bytes are not a real library — these tests only exercise
/// the code path up to the point where the ABI has been decided.
fn fake_plugin(dir: &Path, stem: &str, manifest: &str) -> PathBuf {
    let lib = dir.join(format!("{stem}.{}", plugin_system::library_extension()));
    std::fs::write(&lib, b"not a real library").unwrap();
    std::fs::write(dir.join(format!("{stem}.manifest.json")), manifest).unwrap();
    lib
}

#[test]
fn wasm_manifest_reports_unsupported_abi_when_runtime_is_absent() {
    let temp = TempDir::new().unwrap();
    let lib = fake_plugin(
        temp.path(),
        "libwasmy",
        r#"{"name":"wasmy","version":"0.1.0","abi":"wasm-component"}"#,
    );

    let mut manager = PluginManager::new();
    let err = manager.load_plugin(&lib).unwrap_err();
    let msg = err.to_string();

    // The failure must name the ABI and the plugin, not surface as a generic
    // "missing symbol" error from the native path.
    assert!(
        msg.contains("wasm-component") && msg.contains("wasmy"),
        "expected an unsupported-ABI error naming the plugin, got: {msg}"
    );
    assert!(
        !msg.contains("Symbol"),
        "wasm plugin must not fall through to native symbol resolution: {msg}"
    );
}

#[test]
fn unknown_abi_is_rejected_instead_of_being_loaded_natively() {
    let temp = TempDir::new().unwrap();
    let lib = fake_plugin(
        temp.path(),
        "libmystery",
        r#"{"name":"mystery","version":"0.1.0","abi":"quantum"}"#,
    );

    let mut manager = PluginManager::new();
    let err = manager.load_plugin(&lib).unwrap_err();
    let msg = err.to_string();

    // Guessing wrong here means calling a foreign binary with the wrong
    // calling convention, so an unknown ABI must be a hard error.
    assert!(
        msg.contains("quantum"),
        "error should name the unrecognised abi, got: {msg}"
    );
}

#[test]
fn manifest_without_abi_field_still_takes_the_native_path() {
    let temp = TempDir::new().unwrap();
    let lib = fake_plugin(
        temp.path(),
        "libclassic",
        r#"{"name":"classic","version":"0.1.0","authors":[],"dependencies":[]}"#,
    );

    let mut manager = PluginManager::new();
    let err = manager.load_plugin(&lib).unwrap_err();
    let msg = err.to_string();

    // Pre-existing manifests have no `abi` key; they must keep loading as
    // native. The bytes are junk, so we expect a library-load failure —
    // which proves we reached the native loader.
    assert!(
        msg.contains("Failed to load library") || msg.contains("classic"),
        "expected the native loader to be reached, got: {msg}"
    );
    assert!(
        !msg.contains("unknown abi"),
        "absent abi must default to native, got: {msg}"
    );
}

#[test]
fn abi_round_trips_through_the_manifest() {
    let temp = TempDir::new().unwrap();
    let lib = fake_plugin(
        temp.path(),
        "libcaps",
        r#"{
            "name": "caps", "version": "2.0.0",
            "abi": "wasm-component",
            "interfaces": ["Audio"],
            "capabilities": ["audio"],
            "limits": { "memory_mb": 16, "call_timeout_ms": 250 }
        }"#,
    );

    let manifest = plugin_system::load_plugin_manifest(&lib).unwrap().unwrap();
    assert_eq!(manifest.abi, Abi::WasmComponent);
    assert!(manifest.grants("audio"));
    assert!(!manifest.grants("input"));
    assert_eq!(manifest.limits.memory_mb, 16);
    assert_eq!(manifest.limits.call_timeout_ms, 250);
}

#[test]
fn missing_manifest_is_not_an_error() {
    let temp = TempDir::new().unwrap();
    let lib = temp.path().join("libbare.so");
    std::fs::write(&lib, b"not a real library").unwrap();

    // A native Rust plugin reports metadata through exported symbols and has
    // no sidecar at all; that must stay legal.
    assert!(plugin_system::load_plugin_manifest(&lib).unwrap().is_none());
}
