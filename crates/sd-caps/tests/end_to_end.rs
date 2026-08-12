//! Ported plugins driving the real native backends.
//!
//! These are the tests that prove the migration actually works: a sandboxed
//! component, loaded through `PluginManager`, reaching real hardware through a
//! granted capability. Everything else is unit-level.
//!
//! They read the machine they run on, so they assert on invariants rather than
//! values, and they skip themselves when the component has not been built or
//! the host has no backend to talk to.

use std::path::PathBuf;
use std::sync::Arc;

use plugin_system::{HostCapabilities, PluginManager};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Read a built component, or `None` if the wasm target has not been compiled.
fn component(crate_dir: &str, artifact: &str) -> Option<Vec<u8>> {
    std::fs::read(
        workspace_root()
            .join("plugins")
            .join(crate_dir)
            .join("target/wasm32-wasip2/release")
            .join(artifact),
    )
    .ok()
}

/// Stage a component with a manifest granting `capabilities`, and load it.
fn load(
    dir: &tempfile::TempDir,
    stem: &str,
    bytes: &[u8],
    name: &str,
    capabilities: &str,
    caps: HostCapabilities,
) -> PluginManager {
    let path = dir.path().join(format!("{stem}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    std::fs::write(
        dir.path().join(format!("{stem}.manifest.json")),
        format!(
            r#"{{"name":"{name}","version":"0.1.0","abi":"wasm-component","capabilities":[{capabilities}]}}"#
        ),
    )
    .unwrap();

    let mut manager = PluginManager::new();
    manager.set_capabilities(caps);
    manager.load_plugin(&path).unwrap();
    manager
}

#[test]
fn system_monitor_reports_this_machine() {
    let Some(bytes) = component(
        "plugin-system-monitor-wasm",
        "plugin_system_monitor_wasm.wasm",
    ) else {
        eprintln!("skipping: system-monitor component not built");
        return;
    };

    let temp = tempfile::tempdir().unwrap();
    let caps = HostCapabilities::new().with_system_info(Arc::new(
        sd_caps::system_info::SysinfoProvider::new(),
    ));
    let manager = load(
        &temp,
        "plugin_system_monitor",
        &bytes,
        "system-monitor",
        r#""system-info""#,
        caps,
    );

    let data = manager
        .with_plugin("system-monitor", |p| p.interface_data())
        .unwrap()
        .expect("interface-data should be present after on-load");

    assert!(
        data["cpu_cores"].as_u64().unwrap() >= 1,
        "expected at least one core, got: {data}"
    );
    assert!(
        data["memory_total"].as_u64().unwrap() > 0,
        "expected some memory, got: {data}"
    );
    let usage = data["memory_usage"].as_f64().unwrap();
    assert!(
        (0.0..=100.0).contains(&usage),
        "memory_usage out of range: {usage}"
    );
}

#[test]
fn volume_master_reports_the_real_audio_device() {
    let Some(bytes) = component(
        "plugin-volume-master-wasm",
        "plugin_volume_master_wasm.wasm",
    ) else {
        eprintln!("skipping: volume-master component not built");
        return;
    };

    let provider = Arc::new(sd_caps::audio::NativeAudioProvider::new());
    let support = plugin_system::AudioProvider::support(provider.as_ref());
    if !support.master {
        eprintln!("skipping: no audio backend available on this host");
        return;
    }

    let temp = tempfile::tempdir().unwrap();
    let caps = HostCapabilities::new().with_audio(provider);
    let manager = load(
        &temp,
        "plugin_volume_master",
        &bytes,
        "volume-master",
        r#""audio""#,
        caps,
    );

    let data = manager
        .with_plugin("volume-master", |p| p.interface_data())
        .unwrap()
        .expect("interface-data should be present after on-load");

    assert_eq!(data["platform_supported"], true);

    let volume = data["state"]["master_volume"].as_f64().unwrap();
    assert!(
        (0.0..=100.0).contains(&volume),
        "master volume out of range: {volume}"
    );
    assert!(
        data["state"]["default_device_name"].is_string(),
        "expected a device name, got: {data}"
    );
    assert!(data["apps"].is_array(), "apps should always be an array");
}

/// Without the grant, the same component gets nothing — even though the host
/// has a working backend right there.
#[test]
fn volume_master_without_the_grant_reports_unsupported() {
    let Some(bytes) = component(
        "plugin-volume-master-wasm",
        "plugin_volume_master_wasm.wasm",
    ) else {
        eprintln!("skipping: volume-master component not built");
        return;
    };

    let temp = tempfile::tempdir().unwrap();
    let caps = HostCapabilities::new()
        .with_audio(Arc::new(sd_caps::audio::NativeAudioProvider::new()));
    // Note the empty capability list.
    let manager = load(&temp, "plugin_volume_master", &bytes, "volume-master", "", caps);

    let data = manager
        .with_plugin("volume-master", |p| p.interface_data())
        .unwrap()
        .unwrap();

    assert_eq!(
        data["platform_supported"], false,
        "an ungranted plugin must see audio as unsupported, got: {data}"
    );
    assert_eq!(data["state"]["master_volume"], 0.0);
}
