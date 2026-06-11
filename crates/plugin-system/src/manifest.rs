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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
