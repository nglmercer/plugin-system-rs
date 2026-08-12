//! The one place a plugin name is derived from a file path.
//!
//! Three copies of this rule used to exist — one in `manager.rs`, one in
//! `sd-plugins`, and the `name` field of a manifest — and they did not agree.
//! `plugin_volume_master_wasm.wasm` derived to `volume_master_wasm` in one
//! place while its manifest called it `volume-master`, and the code worked
//! only because the load path happened to prefer the manifest while the
//! enable/disable state matched on the file stem. Two names for one plugin is
//! a bug waiting for the first plugin that ships without a manifest.
//!
//! So there is one function, and everything that needs a name from a path
//! calls it.
//!
//! # What this is *not*
//!
//! It is not the authority on a plugin's identity. A loaded component reports
//! its own name through `get-metadata`, and a manifest states one; either
//! overrides this. This is the fallback for a file nobody has asked yet, and
//! for matching persisted state against a file on disk.

use std::path::Path;

/// The plugin name implied by a file stem.
///
/// Strips the `plugin_`/`plugin-` prefix the build tooling adds and the
/// `_wasm`/`-wasm` suffix the crate names carry, then normalises separators to
/// `-`. `plugin_volume_master_wasm` becomes `volume-master`, which is exactly
/// what that plugin's manifest calls itself.
///
/// Hyphens rather than underscores because that is what the shipped manifests
/// use, and the manifest is the name a user sees.
pub fn canonical_plugin_name(stem: &str) -> String {
    let name = stem
        .strip_prefix("plugin_")
        .or_else(|| stem.strip_prefix("plugin-"))
        .unwrap_or(stem);

    let name = name
        .strip_suffix("_wasm")
        .or_else(|| name.strip_suffix("-wasm"))
        .unwrap_or(name);

    if name.is_empty() {
        // Stripping left nothing — the file really is called `plugin.wasm`.
        // Returning the original stem beats returning an unnameable plugin.
        return stem.to_string();
    }

    name.replace('_', "-")
}

/// The plugin name implied by a file path, or `None` when the path has no
/// usable stem.
pub fn canonical_plugin_name_from_path(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    if stem.is_empty() {
        return None;
    }
    Some(canonical_plugin_name(stem))
}

/// The file stem the build tooling gives a plugin named `name`.
///
/// The inverse of [`canonical_plugin_name`] for the names the tooling itself
/// produces: `volume-master` becomes `plugin_volume_master`.
pub fn plugin_file_stem(name: &str) -> String {
    format!("plugin_{}", name.replace('-', "_"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The case that exposed the drift: this file's manifest says
    /// `volume-master`, and derivation must agree.
    #[test]
    fn a_built_artifact_derives_the_name_its_manifest_uses() {
        assert_eq!(
            canonical_plugin_name("plugin_volume_master_wasm"),
            "volume-master"
        );
        assert_eq!(canonical_plugin_name("plugin_timer_wasm"), "timer");
        assert_eq!(
            canonical_plugin_name("plugin_key_simulator_wasm"),
            "key-simulator"
        );
        assert_eq!(canonical_plugin_name("plugin_obs_wasm"), "obs");
        assert_eq!(
            canonical_plugin_name("plugin_system_monitor_wasm"),
            "system-monitor"
        );
    }

    #[test]
    fn both_prefix_spellings_are_stripped() {
        assert_eq!(canonical_plugin_name("plugin-timer"), "timer");
        assert_eq!(canonical_plugin_name("plugin_timer"), "timer");
        assert_eq!(canonical_plugin_name("timer"), "timer");
    }

    /// The `lib` prefix was a native-linker convention. A component is named
    /// whatever the build produced, so `lib` is now part of the name.
    #[test]
    fn the_lib_prefix_is_not_stripped() {
        assert_eq!(canonical_plugin_name("libplugin_timer"), "libplugin-timer");
    }

    #[test]
    fn a_stem_that_is_only_a_prefix_keeps_its_stem() {
        assert_eq!(canonical_plugin_name("plugin_"), "plugin_");
        // Prefix and suffix are stripped independently, so this leaves `wasm`
        // rather than nothing.
        assert_eq!(canonical_plugin_name("plugin-wasm"), "wasm");
    }

    #[test]
    fn round_trips_through_the_build_tooling_naming() {
        for name in ["timer", "volume-master", "system-monitor"] {
            let stem = plugin_file_stem(name);
            assert_eq!(canonical_plugin_name(&stem), name, "round-tripping {name}");
        }
    }

    #[test]
    fn derives_from_a_path() {
        let path = Path::new("/opt/plugins/plugin_volume_master_wasm.wasm");
        assert_eq!(
            canonical_plugin_name_from_path(path).as_deref(),
            Some("volume-master")
        );
        assert!(canonical_plugin_name_from_path(Path::new("")).is_none());
    }
}
