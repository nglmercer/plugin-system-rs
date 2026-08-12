use std::path::{Path, PathBuf};

use crate::traits::{PluginDependency, PluginMetadata};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManifestDependency {
    pub name: String,
    pub version_req: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub dependencies: Vec<ManifestDependency>,
}

impl From<Manifest> for PluginMetadata {
    fn from(manifest: Manifest) -> Self {
        PluginMetadata {
            name: manifest.name,
            version: manifest.version,
            authors: manifest.authors,
            dependencies: manifest
                .dependencies
                .into_iter()
                .map(|dep| PluginDependency {
                    name: dep.name,
                    version_req: dep.version_req,
                })
                .collect(),
        }
    }
}

/// Which calling convention the host must use to talk to a plugin binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Abi {
    /// Rust trait object passed across a `cdylib` boundary. Only sound when
    /// host and plugin are built by the same compiler with the same
    /// dependency versions. Default for manifests that do not say otherwise.
    #[default]
    #[serde(rename = "native")]
    Native,
    /// Flat C ABI: opaque handle plus JSON in/out. Stable across compilers.
    #[serde(rename = "c-flat", alias = "c_abi")]
    CFlat,
    /// WebAssembly component running on WASI Preview 2, sandboxed and
    /// portable across every host platform.
    #[serde(rename = "wasm-component", alias = "wasm")]
    WasmComponent,
}

impl Abi {
    /// Parse the `abi` field of a manifest. Unknown values are rejected
    /// rather than silently treated as native, since guessing wrong means
    /// calling a foreign binary with the wrong convention.
    pub fn parse(s: &str) -> std::result::Result<Self, String> {
        match s.to_ascii_lowercase().replace('_', "-").as_str() {
            "native" | "rust" => Ok(Abi::Native),
            "c-flat" | "c-abi" => Ok(Abi::CFlat),
            "wasm-component" | "wasm" | "component" => Ok(Abi::WasmComponent),
            other => Err(format!("unknown abi `{other}`")),
        }
    }

    /// Whether this ABI loads a native shared library into the host process.
    pub fn is_native_library(&self) -> bool {
        matches!(self, Abi::Native | Abi::CFlat)
    }
}

/// Per-plugin resource ceilings. Only enforceable for sandboxed ABIs; the
/// native paths parse these but cannot honour them.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResourceLimits {
    /// Maximum linear memory the plugin may allocate, in mebibytes.
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u64,
    /// Wall-clock budget for a single call before the instance is trapped.
    #[serde(default = "default_call_timeout_ms")]
    pub call_timeout_ms: u64,
}

fn default_memory_mb() -> u64 {
    64
}

fn default_call_timeout_ms() -> u64 {
    5_000
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: default_memory_mb(),
            call_timeout_ms: default_call_timeout_ms(),
        }
    }
}

/// The full sidecar descriptor (`<plugin>.manifest.json`).
///
/// This supersedes the C-ABI-specific manifest: one type describes every ABI,
/// so the loader can dispatch on `abi` instead of sniffing for a magic value.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<ManifestDependency>,
    /// How the host should call into this plugin.
    #[serde(default, deserialize_with = "deserialize_abi")]
    pub abi: Abi,
    /// Interface ids the plugin implements, for ABIs that cannot report them
    /// from code at load time.
    #[serde(default)]
    pub interfaces: Vec<String>,
    /// Host capabilities the plugin requests. Anything not listed here is not
    /// granted at instantiation.
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub limits: ResourceLimits,
    /// Optional pre-load configuration handed to the plugin.
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

fn deserialize_abi<'de, D>(deserializer: D) -> std::result::Result<Abi, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize as _;
    match Option::<String>::deserialize(deserializer)? {
        Some(s) => Abi::parse(&s).map_err(serde::de::Error::custom),
        None => Ok(Abi::Native),
    }
}

impl PluginManifest {
    /// Whether the plugin asked for a given host capability.
    pub fn grants(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
}

impl From<PluginManifest> for PluginMetadata {
    fn from(manifest: PluginManifest) -> Self {
        PluginMetadata {
            name: manifest.name,
            version: manifest.version,
            authors: manifest.authors,
            dependencies: manifest
                .dependencies
                .into_iter()
                .map(|dep| PluginDependency {
                    name: dep.name,
                    version_req: dep.version_req,
                })
                .collect(),
        }
    }
}

pub fn manifest_path(lib_path: impl AsRef<Path>) -> PathBuf {
    let lib_path = lib_path.as_ref();
    let stem = lib_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let parent = lib_path.parent().unwrap_or(Path::new(""));
    parent.join(format!("{}.manifest.json", stem))
}

pub fn load_manifest(lib_path: impl AsRef<Path>) -> Result<Option<Manifest>, std::io::Error> {
    let manifest_path = manifest_path(lib_path);
    if !manifest_path.exists() {
        return Ok(None);
    }
    let data = std::fs::read(&manifest_path)?;
    let manifest: Manifest = serde_json::from_slice(&data).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "failed to parse manifest {}: {}",
                manifest_path.display(),
                e
            ),
        )
    })?;
    Ok(Some(manifest))
}

/// Read the sidecar descriptor next to a plugin binary, whatever its ABI.
///
/// Returns `Ok(None)` when no manifest exists — that is the normal case for a
/// native Rust plugin, which reports its metadata through exported symbols.
pub fn load_plugin_manifest(
    lib_path: impl AsRef<Path>,
) -> Result<Option<PluginManifest>, std::io::Error> {
    let manifest_path = manifest_path(&lib_path);
    if !manifest_path.exists() {
        return Ok(None);
    }
    let data = std::fs::read(&manifest_path)?;
    let manifest: PluginManifest = serde_json::from_slice(&data).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "failed to parse manifest {}: {}",
                manifest_path.display(),
                e
            ),
        )
    })?;
    Ok(Some(manifest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_manifest(dir: &Path, stem: &str, body: &str) -> PathBuf {
        let lib_path = dir.join(format!("{stem}.so"));
        fs::write(&lib_path, b"fake").unwrap();
        fs::write(manifest_path(&lib_path), body).unwrap();
        lib_path
    }

    #[test]
    fn abi_defaults_to_native_when_absent() {
        let temp = tempfile::tempdir().unwrap();
        let lib = write_manifest(
            temp.path(),
            "libplain",
            r#"{"name":"plain","version":"1.0.0"}"#,
        );
        let manifest = load_plugin_manifest(&lib).unwrap().unwrap();
        assert_eq!(manifest.abi, Abi::Native);
        assert!(manifest.capabilities.is_empty());
        assert_eq!(manifest.limits, ResourceLimits::default());
    }

    #[test]
    fn parses_wasm_manifest_with_capabilities_and_limits() {
        let temp = tempfile::tempdir().unwrap();
        let lib = write_manifest(
            temp.path(),
            "libobs",
            r#"{
                "name": "obs", "version": "0.1.0",
                "abi": "wasm-component",
                "interfaces": ["ObsControl"],
                "capabilities": ["websocket", "config"],
                "limits": { "memory_mb": 32, "call_timeout_ms": 1000 }
            }"#,
        );
        let manifest = load_plugin_manifest(&lib).unwrap().unwrap();
        assert_eq!(manifest.abi, Abi::WasmComponent);
        assert_eq!(manifest.interfaces, vec!["ObsControl"]);
        assert!(manifest.grants("websocket"));
        assert!(!manifest.grants("audio"));
        assert_eq!(manifest.limits.memory_mb, 32);
        assert_eq!(manifest.limits.call_timeout_ms, 1000);
    }

    #[test]
    fn limits_fall_back_to_defaults_field_by_field() {
        let temp = tempfile::tempdir().unwrap();
        let lib = write_manifest(
            temp.path(),
            "libpartial",
            r#"{"name":"p","version":"1.0.0","abi":"wasm","limits":{"memory_mb":8}}"#,
        );
        let manifest = load_plugin_manifest(&lib).unwrap().unwrap();
        assert_eq!(manifest.limits.memory_mb, 8);
        assert_eq!(manifest.limits.call_timeout_ms, 5_000);
    }

    #[test]
    fn c_flat_aliases_are_accepted() {
        for raw in ["c-flat", "c_abi", "C-Flat"] {
            assert_eq!(Abi::parse(raw).unwrap(), Abi::CFlat, "parsing {raw}");
        }
    }

    #[test]
    fn unknown_abi_is_rejected_rather_than_assumed_native() {
        let temp = tempfile::tempdir().unwrap();
        let lib = write_manifest(
            temp.path(),
            "libweird",
            r#"{"name":"w","version":"1.0.0","abi":"telepathy"}"#,
        );
        let err = load_plugin_manifest(&lib).unwrap_err();
        assert!(
            err.to_string().contains("telepathy"),
            "error should name the bad abi, got: {err}"
        );
    }

    #[test]
    fn wasm_abi_is_not_a_native_library() {
        assert!(Abi::Native.is_native_library());
        assert!(Abi::CFlat.is_native_library());
        assert!(!Abi::WasmComponent.is_native_library());
    }

    #[test]
    fn test_load_manifest_returns_none_when_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let lib_path = temp_dir.path().join("libmyplugin.so");
        fs::write(&lib_path, b"fake").unwrap();
        let result = load_manifest(&lib_path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_manifest_parses_correctly() {
        let temp_dir = tempfile::tempdir().unwrap();
        let lib_path = temp_dir.path().join("libmyplugin.so");
        fs::write(&lib_path, b"fake").unwrap();
        let manifest_content = r#"{
            "name": "myplugin",
            "version": "1.0.0",
            "authors": ["Alice", "Bob"],
            "dependencies": [
                {"name": "core", "version_req": ">= 2.0"},
                {"name": "utils", "version_req": "~1.4"}
            ]
        }"#;
        let manifest_path = manifest_path(&lib_path);
        fs::write(&manifest_path, manifest_content).unwrap();
        let result = load_manifest(&lib_path).unwrap();
        let manifest = result.unwrap();
        assert_eq!(manifest.name, "myplugin");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.authors, vec!["Alice", "Bob"]);
        assert_eq!(manifest.dependencies.len(), 2);
        assert_eq!(manifest.dependencies[0].name, "core");
        assert_eq!(manifest.dependencies[0].version_req, ">= 2.0");
        assert_eq!(manifest.dependencies[1].name, "utils");
        assert_eq!(manifest.dependencies[1].version_req, "~1.4");
    }

    #[test]
    fn test_manifest_path_construction() {
        let lib_path = Path::new("/opt/plugins/libfoo.dylib");
        let expected = PathBuf::from("/opt/plugins/libfoo.manifest.json");
        assert_eq!(manifest_path(lib_path), expected);
    }

    #[test]
    fn test_manifest_path_windows() {
        let lib_path = Path::new("C:\\plugins\\myplugin.dll");
        let expected = PathBuf::from("C:\\plugins\\myplugin.manifest.json");
        assert_eq!(manifest_path(lib_path), expected);
    }

    #[test]
    fn test_load_manifest_invalid_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        let lib_path = temp_dir.path().join("libbad.so");
        fs::write(&lib_path, b"fake").unwrap();
        let manifest_path = manifest_path(&lib_path);
        fs::write(&manifest_path, b"{ invalid json }").unwrap();
        let result = load_manifest(&lib_path);
        assert!(result.is_err());
    }
}
