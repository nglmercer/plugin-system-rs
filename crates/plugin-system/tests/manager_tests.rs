//! `PluginManager` behaviour that does not depend on a real component.
//!
//! Tests that actually instantiate a guest live in `wasm_tests.rs`; these pin
//! the surrounding loader contract — what counts as a plugin file, what a
//! missing or malformed manifest does, and what happens to the ABIs that were
//! removed.
//!
//! Most of what used to be in this file measured free space in `/tmp` and
//! rehearsed the write-to-temp-file step, because `dlopen` could only open a
//! real path and the spill was a genuine failure mode. wasmtime compiles from
//! a byte slice, so there is no temp file left to run out of room for.

use plugin_system::{PluginError, PluginManager, PLUGIN_EXTENSION};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Write a file that is named like a plugin but is not a component, plus an
/// optional sidecar manifest. Enough to drive the loader up to the point where
/// wasmtime rejects the bytes.
fn fake_component(dir: &Path, stem: &str, manifest: Option<&str>) -> PathBuf {
    let path = dir.join(format!("{stem}.{PLUGIN_EXTENSION}"));
    std::fs::write(&path, b"not a real component").unwrap();
    if let Some(body) = manifest {
        std::fs::write(dir.join(format!("{stem}.manifest.json")), body).unwrap();
    }
    path
}

#[test]
fn test_manager_new_creates_empty_registry() {
    let manager = PluginManager::new();
    assert!(manager.plugin_names().is_empty());
}

#[test]
fn test_load_plugins_from_dir_empty() {
    let mut manager = PluginManager::new();
    let temp_dir = TempDir::new().unwrap();
    let loaded = manager.load_plugins_from_dir(temp_dir.path()).unwrap();
    assert!(loaded.is_empty());
}

/// A directory scan must ignore anything that is not a `.wasm`.
///
/// This used to be the opposite: files with no extension were accepted
/// because a native library could be named anything. Leftover `.so` files
/// from a pre-migration install must now be skipped in silence rather than
/// attempted and logged as failures.
#[test]
fn load_plugins_from_dir_ignores_non_component_files() {
    let temp = TempDir::new().unwrap();
    for name in [
        "libplugin_timer.so",
        "plugin_timer.dll",
        "libplugin_timer.dylib",
        "README",
        "plugin_timer.manifest.json",
    ] {
        std::fs::write(temp.path().join(name), b"stale artifact").unwrap();
    }

    let mut manager = PluginManager::new();
    let loaded = manager.load_plugins_from_dir(temp.path()).unwrap();

    assert!(
        loaded.is_empty(),
        "no component present, so nothing should load: {loaded:?}"
    );
}

/// Bytes that are not a component must be rejected as such.
#[test]
fn loading_bytes_that_are_not_a_component_fails() {
    let temp = TempDir::new().unwrap();
    let path = fake_component(
        temp.path(),
        "plugin_bogus",
        Some(r#"{"name":"bogus","version":"0.1.0","abi":"wasm-component"}"#),
    );

    let mut manager = PluginManager::new();
    let err = manager.load_plugin(&path).unwrap_err();

    assert!(
        !manager.is_loaded("bogus"),
        "a plugin that failed to load must not be registered"
    );
    // The error should come from the component runtime, not from something
    // that looks like a missing file.
    assert!(
        !matches!(err, PluginError::PluginNotFound { .. }),
        "expected a load failure, got: {err}"
    );
}

/// A manifest is optional. Absent one, the loader still treats the file as a
/// component rather than falling through to some other backend — there is no
/// other backend to fall through to.
#[test]
fn a_component_without_a_manifest_is_still_loaded_as_a_component() {
    let temp = TempDir::new().unwrap();
    let path = fake_component(temp.path(), "plugin_undeclared", None);

    let mut manager = PluginManager::new();
    let err = manager.load_plugin(&path).unwrap_err().to_string();

    // The loader got as far as handing the bytes to wasmtime, which is the
    // proof that the absent manifest was not itself the obstacle.
    assert!(
        err.contains("WebAssembly"),
        "expected a component-validation failure, got: {err}"
    );
    // Name falls back to the file stem with the `plugin_` prefix stripped.
    assert!(
        err.contains("undeclared"),
        "error should name the plugin, got: {err}"
    );
}

/// A manifest carried over from the FFI era must fail with an explanation
/// rather than being quietly reinterpreted.
#[test]
fn a_manifest_declaring_a_retired_native_abi_is_rejected() {
    for abi in ["native", "c-flat"] {
        let temp = TempDir::new().unwrap();
        let path = fake_component(
            temp.path(),
            "plugin_legacy",
            Some(&format!(
                r#"{{"name":"legacy","version":"0.1.0","abi":"{abi}"}}"#
            )),
        );

        let mut manager = PluginManager::new();
        let err = manager.load_plugin(&path).unwrap_err().to_string();

        assert!(
            err.contains("was removed"),
            "abi `{abi}` should be reported as removed, got: {err}"
        );
    }
}

/// Listing a plugin without instantiating it relies on the sidecar manifest.
#[test]
fn metadata_from_path_reads_the_sidecar_manifest() {
    let temp = TempDir::new().unwrap();
    let path = fake_component(
        temp.path(),
        "plugin_described",
        Some(r#"{"name":"described","version":"2.1.0","authors":["Ada"]}"#),
    );

    let metadata = PluginManager::metadata_from_path(&path).unwrap();
    assert_eq!(metadata.name, "described");
    assert_eq!(metadata.version, "2.1.0");
    assert_eq!(metadata.authors, vec!["Ada"]);
}

#[test]
fn metadata_from_path_without_a_manifest_says_so() {
    let temp = TempDir::new().unwrap();
    let path = fake_component(temp.path(), "plugin_bare", None);

    let err = PluginManager::metadata_from_path(&path).unwrap_err().to_string();
    assert!(
        err.contains("manifest"),
        "error should point at the missing manifest, got: {err}"
    );
}

#[test]
fn unloading_a_plugin_that_was_never_loaded_reports_not_found() {
    let mut manager = PluginManager::new();
    let err = manager.unload_plugin("ghost").unwrap_err();
    assert!(matches!(err, PluginError::PluginNotFound { .. }), "got: {err}");
}
