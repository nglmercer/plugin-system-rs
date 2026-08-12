//! Admission policy for plugins arriving over the API.
//!
//! A plugin is a WebAssembly component, which means the sandbox contains it —
//! but the sandbox is only as tight as the capabilities it is granted, and
//! those come from a manifest the uploader supplies. A plugin that asks for
//! `input` gets synthetic keystrokes into whatever window has focus *and* a
//! global view of everything typed. Loading that because someone POSTed a file
//! is not a plugin system, it is a remote shell with extra steps.
//!
//! So an upload has to pass three checks before anything is written to disk:
//!
//!  1. Every capability the manifest requests must be **acknowledged** by the
//!     caller, by name. Nothing is granted implicitly, and a caller that does
//!     not know what it is installing cannot install it.
//!  2. Capabilities in [`REQUIRES_HOST_OPT_IN`] additionally need an
//!     environment opt-in on the host. An API acknowledgement is only as
//!     trustworthy as the API's own authentication; the riskiest grants want a
//!     decision made at the machine, not over the wire.
//!  3. If an allowlist file exists, the uploaded bytes must hash to an entry
//!     in it. Absent the file there is no allowlist — but where one exists it
//!     is authoritative, which gives a locked-down install a way to pin
//!     exactly which binaries may ever be loaded.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use plugin_system::{capabilities as caps, PluginManifest};
use sha2::{Digest, Sha256};

use crate::{PluginResult, PluginResultError};

/// Capabilities that an acknowledgement alone is not enough to grant.
///
/// Just `input` today. It is the one capability whose abuse is indistinguishable
/// from the user sitting at the keyboard, so granting it to a binary that
/// arrived over the network is a decision that belongs to whoever runs the
/// daemon.
pub const REQUIRES_HOST_OPT_IN: &[&str] = &[caps::INPUT];

/// Environment variable that opts a host into granting `input` to uploads.
pub const INPUT_OPT_IN_ENV: &str = "SD_ALLOW_UPLOADED_INPUT_CAPABILITY";

/// Filename of the optional hash allowlist, read from the plugin directory.
pub const ALLOWLIST_FILE: &str = "allowed-plugins.json";

/// An uploaded plugin, before anything has been written to disk.
pub struct PluginUpload<'a> {
    pub bytes: &'a [u8],
    pub filename: &'a str,
    /// The sidecar manifest, as JSON. `None` means the plugin ships no
    /// manifest, which is the maximally constrained case: no capabilities, no
    /// declared interfaces, default limits.
    pub manifest_json: Option<&'a str>,
    /// Capability names the caller explicitly agreed to grant.
    pub acknowledged_capabilities: &'a [String],
    /// Whether to enable the plugin after installing. `None` means "leave it
    /// as it was", which only has a meaning on update; a fresh install treats
    /// it as enabled.
    pub enabled: Option<bool>,
}

/// Parse the manifest an upload carries, or synthesise the empty one.
pub fn parse_upload_manifest(
    manifest_json: Option<&str>,
    fallback_name: &str,
) -> PluginResult<PluginManifest> {
    match manifest_json {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw).map_err(|e| {
            PluginResultError::InvalidInput(format!("Plugin manifest is not valid: {e}"))
        }),
        _ => Ok(PluginManifest::for_component(fallback_name)),
    }
}

/// Reject an upload whose manifest asks for something the caller did not
/// agree to, the host has not opted into, or that this crate cannot grant.
pub fn authorize_capabilities(
    manifest: &PluginManifest,
    acknowledged: &[String],
) -> PluginResult<()> {
    let acknowledged: HashSet<&str> = acknowledged.iter().map(String::as_str).collect();

    // A misspelled capability would otherwise be silently never granted, and
    // the plugin would fail at runtime with a confusing "not granted" error.
    let unknown: Vec<&str> = manifest
        .capabilities
        .iter()
        .map(String::as_str)
        .filter(|name| !caps::is_known(name))
        .collect();
    if !unknown.is_empty() {
        return Err(PluginResultError::InvalidInput(format!(
            "Plugin manifest requests unknown capabilities: {}. Known capabilities: {}.",
            unknown.join(", "),
            caps::ALL.join(", ")
        )));
    }

    let unacknowledged: Vec<&str> = manifest
        .capabilities
        .iter()
        .map(String::as_str)
        .filter(|name| !acknowledged.contains(name))
        .collect();
    if !unacknowledged.is_empty() {
        return Err(PluginResultError::CapabilitiesNotAcknowledged {
            requested: manifest.capabilities.clone(),
            missing: unacknowledged.iter().map(|s| s.to_string()).collect(),
        });
    }

    for name in &manifest.capabilities {
        if REQUIRES_HOST_OPT_IN.contains(&name.as_str()) && !host_opted_in(name) {
            return Err(PluginResultError::CapabilityRefused {
                capability: name.clone(),
                reason: format!(
                    "'{name}' grants global keystroke injection and a global view of what \
                     is typed, so it is never granted to an uploaded plugin by default. \
                     Set {INPUT_OPT_IN_ENV}=1 on the host to allow it, or install the \
                     plugin by copying it into the plugin directory."
                ),
            });
        }
    }

    Ok(())
}

fn host_opted_in(capability: &str) -> bool {
    if capability != caps::INPUT {
        return false;
    }
    std::env::var(INPUT_OPT_IN_ENV)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Lowercase hex SHA-256 of the uploaded bytes.
pub fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Reject an upload whose hash is not in the allowlist, when one exists.
pub fn authorize_digest(plugin_dir: &Path, bytes: &[u8]) -> PluginResult<()> {
    let path = allowlist_path(plugin_dir);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        // No allowlist configured. Not an error: pinning binaries is opt-in.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(PluginResultError::Io(e.to_string())),
    };

    let allowed: Vec<String> = serde_json::from_str(&raw).map_err(|e| {
        PluginResultError::InvalidInput(format!(
            "{} is not a JSON array of sha256 hashes: {e}",
            path.display()
        ))
    })?;

    let actual = digest(bytes);
    if allowed
        .iter()
        .any(|entry| entry.trim().eq_ignore_ascii_case(&actual))
    {
        return Ok(());
    }

    Err(PluginResultError::InvalidInput(format!(
        "Plugin sha256 {actual} is not listed in {}. Add it there to allow this binary.",
        path.display()
    )))
}

fn allowlist_path(plugin_dir: &Path) -> PathBuf {
    plugin_dir.join(ALLOWLIST_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_with(capabilities: &[&str]) -> PluginManifest {
        let mut manifest = PluginManifest::for_component("test");
        manifest.capabilities = capabilities.iter().map(|s| s.to_string()).collect();
        manifest
    }

    #[test]
    fn a_plugin_asking_for_nothing_needs_no_acknowledgement() {
        assert!(authorize_capabilities(&manifest_with(&[]), &[]).is_ok());
    }

    /// The core rule: capabilities are never granted by default.
    #[test]
    fn unacknowledged_capabilities_are_refused() {
        let err = authorize_capabilities(&manifest_with(&["audio"]), &[]).unwrap_err();
        assert!(
            matches!(err, PluginResultError::CapabilitiesNotAcknowledged { .. }),
            "got {err:?}"
        );
        assert!(err.to_string().contains("audio"));
    }

    #[test]
    fn acknowledged_capabilities_pass() {
        assert!(
            authorize_capabilities(&manifest_with(&["audio"]), &["audio".to_string()]).is_ok()
        );
    }

    /// Acknowledging over the API is not enough for the keyboard.
    #[test]
    fn input_needs_a_host_opt_in_even_when_acknowledged() {
        let err = authorize_capabilities(&manifest_with(&["input"]), &["input".to_string()])
            .unwrap_err();
        assert!(
            matches!(err, PluginResultError::CapabilityRefused { .. }),
            "got {err:?}"
        );
        assert!(err.to_string().contains(INPUT_OPT_IN_ENV));
    }

    #[test]
    fn a_misspelled_capability_is_rejected_rather_than_ignored() {
        let err =
            authorize_capabilities(&manifest_with(&["audioo"]), &["audioo".to_string()])
                .unwrap_err();
        assert!(err.to_string().contains("unknown"), "got: {err}");
    }

    #[test]
    fn a_missing_allowlist_allows_everything() {
        let temp = tempfile::tempdir().unwrap();
        assert!(authorize_digest(temp.path(), b"anything").is_ok());
    }

    #[test]
    fn an_allowlist_pins_exactly_the_listed_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let allowed = digest(b"the good plugin");
        std::fs::write(
            temp.path().join(ALLOWLIST_FILE),
            serde_json::to_string(&vec![allowed]).unwrap(),
        )
        .unwrap();

        assert!(authorize_digest(temp.path(), b"the good plugin").is_ok());
        let err = authorize_digest(temp.path(), b"the bad plugin").unwrap_err();
        assert!(err.to_string().contains("not listed"), "got: {err}");
    }

    #[test]
    fn an_absent_manifest_grants_nothing() {
        let manifest = parse_upload_manifest(None, "derived").unwrap();
        assert_eq!(manifest.name, "derived");
        assert!(manifest.capabilities.is_empty());
    }

    #[test]
    fn a_supplied_manifest_is_parsed() {
        let manifest = parse_upload_manifest(
            Some(r#"{"name":"obs","version":"1.0.0","capabilities":["websocket"]}"#),
            "derived",
        )
        .unwrap();
        assert_eq!(manifest.name, "obs");
        assert_eq!(manifest.capabilities, vec!["websocket"]);
    }

    #[test]
    fn a_malformed_manifest_is_an_error_not_a_silent_default() {
        let err = parse_upload_manifest(Some("{ nope }"), "derived").unwrap_err();
        assert!(err.to_string().contains("not valid"), "got: {err}");
    }
}
