//! The sidecar manifest contract, as seen from outside the crate.
//!
//! This file replaces the old `abi_dispatch_tests.rs`, which pinned how the
//! loader chose between the native, c-flat, and wasm-component backends. There
//! is one backend now, so there is nothing to dispatch on — what is left worth
//! pinning is the manifest itself: what it grants, what it limits, and what it
//! does with an `abi` value that no longer exists.

use plugin_system::{load_plugin_manifest, Abi, PLUGIN_EXTENSION};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn write_plugin(dir: &Path, stem: &str, manifest: &str) -> PathBuf {
    let path = dir.join(format!("{stem}.{PLUGIN_EXTENSION}"));
    std::fs::write(&path, b"not a real component").unwrap();
    std::fs::write(dir.join(format!("{stem}.manifest.json")), manifest).unwrap();
    path
}

#[test]
fn capabilities_and_limits_round_trip_through_the_manifest() {
    let temp = TempDir::new().unwrap();
    let path = write_plugin(
        temp.path(),
        "plugin_caps",
        r#"{
            "name": "caps", "version": "2.0.0",
            "abi": "wasm-component",
            "interfaces": ["Audio"],
            "capabilities": ["audio"],
            "limits": { "memory_mb": 16, "call_timeout_ms": 250 }
        }"#,
    );

    let manifest = load_plugin_manifest(&path).unwrap().unwrap();
    assert_eq!(manifest.abi, Abi::WasmComponent);
    assert!(manifest.grants("audio"));
    assert!(!manifest.grants("input"));
    assert_eq!(manifest.limits.memory_mb, 16);
    assert_eq!(manifest.limits.call_timeout_ms, 250);
}

/// A manifest with no `abi` key is a component. Before the migration this
/// defaulted to the native ABI, so the default flipping is the deliberate
/// behaviour change worth a test of its own.
#[test]
fn a_manifest_without_an_abi_field_defaults_to_a_component() {
    let temp = TempDir::new().unwrap();
    let path = write_plugin(
        temp.path(),
        "plugin_classic",
        r#"{"name":"classic","version":"0.1.0","authors":[],"dependencies":[]}"#,
    );

    let manifest = load_plugin_manifest(&path).unwrap().unwrap();
    assert_eq!(manifest.abi, Abi::WasmComponent);
}

#[test]
fn an_unknown_abi_is_rejected() {
    let temp = TempDir::new().unwrap();
    let path = write_plugin(
        temp.path(),
        "plugin_mystery",
        r#"{"name":"mystery","version":"0.1.0","abi":"quantum"}"#,
    );

    let err = load_plugin_manifest(&path).unwrap_err().to_string();
    assert!(
        err.contains("quantum"),
        "error should name the unrecognised abi, got: {err}"
    );
}

/// Manifests written for the FFI backends must fail with a message that says
/// the ABI was removed, not one that says it was never known.
#[test]
fn a_retired_native_abi_is_rejected_with_migration_advice() {
    for abi in ["native", "c-flat", "c_abi"] {
        let temp = TempDir::new().unwrap();
        let path = write_plugin(
            temp.path(),
            "plugin_legacy",
            &format!(r#"{{"name":"legacy","version":"0.1.0","abi":"{abi}"}}"#),
        );

        let err = load_plugin_manifest(&path).unwrap_err().to_string();
        assert!(
            err.contains("was removed") && err.contains("wasm-component"),
            "abi `{abi}` should be reported as removed with a pointer to the \
             replacement, got: {err}"
        );
    }
}

#[test]
fn a_missing_manifest_is_not_an_error() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join(format!("plugin_bare.{PLUGIN_EXTENSION}"));
    std::fs::write(&path, b"not a real component").unwrap();

    // Identity comes from the guest's `get-metadata`; the manifest only adds
    // capability grants and limits on top, so shipping without one is legal.
    assert!(load_plugin_manifest(&path).unwrap().is_none());
}
