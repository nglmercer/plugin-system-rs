use plugin_system::{PluginMetadata, PluginRegistry};

#[test]
fn test_registry_new() {
    let registry = PluginRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
}

#[test]
fn test_registry_default() {
    let registry = PluginRegistry::default();
    assert!(registry.is_empty());
}

#[test]
fn test_plugin_metadata_creation() {
    let metadata = PluginMetadata {
        name: "test_plugin".to_string(),
        version: "0.1.0".to_string(),
        authors: vec!["Author".to_string()],
        dependencies: vec!["dep1".to_string()],
    };

    assert_eq!(metadata.name, "test_plugin");
    assert_eq!(metadata.version, "0.1.0");
    assert_eq!(metadata.authors.len(), 1);
    assert_eq!(metadata.dependencies.len(), 1);
}

#[test]
fn test_plugin_metadata_serialization() {
    let metadata = PluginMetadata {
        name: "test_plugin".to_string(),
        version: "0.2.0".to_string(),
        authors: vec!["Author1".to_string(), "Author2".to_string()],
        dependencies: vec![],
    };

    let json = serde_json::to_string(&metadata).unwrap();
    let deserialized: PluginMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(metadata.name, deserialized.name);
    assert_eq!(metadata.version, deserialized.version);
    assert_eq!(metadata.authors, deserialized.authors);
    assert_eq!(metadata.dependencies, deserialized.dependencies);
}

#[test]
fn test_plugin_metadata_clone() {
    let metadata = PluginMetadata {
        name: "test_plugin".to_string(),
        version: "0.1.0".to_string(),
        authors: vec!["Author".to_string()],
        dependencies: vec!["dep1".to_string()],
    };

    let cloned = metadata.clone();
    assert_eq!(metadata.name, cloned.name);
    assert_eq!(metadata.version, cloned.version);
    assert_eq!(metadata.authors, cloned.authors);
    assert_eq!(metadata.dependencies, cloned.dependencies);
}

#[test]
fn test_plugin_metadata_debug() {
    let metadata = PluginMetadata {
        name: "test_plugin".to_string(),
        version: "0.1.0".to_string(),
        authors: vec![],
        dependencies: vec![],
    };

    let debug_str = format!("{:?}", metadata);
    assert!(debug_str.contains("test_plugin"));
    assert!(debug_str.contains("0.1.0"));
}

#[test]
fn test_plugin_metadata_empty_dependencies() {
    let metadata = PluginMetadata {
        name: "no_deps".to_string(),
        version: "1.0.0".to_string(),
        authors: vec![],
        dependencies: vec![],
    };

    assert!(metadata.dependencies.is_empty());
}

#[test]
fn test_plugin_metadata_multiple_dependencies() {
    let metadata = PluginMetadata {
        name: "multi_deps".to_string(),
        version: "1.0.0".to_string(),
        authors: vec![],
        dependencies: vec![
            "dep1".to_string(),
            "dep2".to_string(),
            "dep3".to_string(),
        ],
    };

    assert_eq!(metadata.dependencies.len(), 3);
    assert!(metadata.dependencies.contains(&"dep1".to_string()));
    assert!(metadata.dependencies.contains(&"dep2".to_string()));
    assert!(metadata.dependencies.contains(&"dep3".to_string()));
}
