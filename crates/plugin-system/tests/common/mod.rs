//! Shared helpers for tests that need a real component on disk.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Read a component built out of `plugins/<crate>`, if it has been built.
///
/// Returns `None` rather than failing so the suite still runs on a checkout
/// where the wasm targets have not been compiled; the tests that need one skip
/// themselves with a note.
fn component(crate_dir: &str, artifact: &str) -> Option<Vec<u8>> {
    let path = workspace_root()
        .join("plugins")
        .join(crate_dir)
        .join("target/wasm32-wasip2/release")
        .join(artifact);
    std::fs::read(path).ok()
}

pub fn monitor_component() -> Option<Vec<u8>> {
    component(
        "plugin-system-monitor-wasm",
        "plugin_system_monitor_wasm.wasm",
    )
}

/// Write a component plus its sidecar manifest, returning the component path.
pub fn write_plugin(dir: &Path, stem: &str, bytes: &[u8], manifest: &str) -> PathBuf {
    let path = dir.join(format!("{stem}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    std::fs::write(dir.join(format!("{stem}.manifest.json")), manifest).unwrap();
    path
}
