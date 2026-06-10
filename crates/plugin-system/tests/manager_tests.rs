use plugin_system::{PluginManager, PluginRegistry};

#[test]
fn test_plugin_manager_new() {
    let manager = PluginManager::new();
    assert!(manager.plugin_names().is_empty());
}

#[test]
fn test_plugin_manager_default() {
    let manager = PluginManager::default();
    assert!(manager.plugin_names().is_empty());
}

#[test]
fn test_plugin_manager_registry() {
    let manager = PluginManager::new();
    let registry = manager.registry();

    // Registry should be shared
    let reg1 = registry.clone();
    let reg2 = manager.registry();

    // Both should reference the same registry
    let names1 = reg1.read().unwrap().plugin_names();
    let names2 = reg2.read().unwrap().plugin_names();
    assert_eq!(names1, names2);
}

#[test]
fn test_plugin_manager_is_loaded() {
    let manager = PluginManager::new();
    assert!(!manager.is_loaded("nonexistent"));
}

#[test]
fn test_plugin_manager_plugin_path() {
    let manager = PluginManager::new();
    assert!(manager.plugin_path("nonexistent").is_none());
}

#[test]
fn test_plugin_manager_plugin_metadata() {
    let manager = PluginManager::new();
    assert!(manager.plugin_metadata("nonexistent").is_none());
}

#[test]
fn test_plugin_manager_load_nonexistent_dir() {
    let mut manager = PluginManager::new();
    let result = manager.load_plugins_from_dir("/nonexistent/path");
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_plugin_manager_load_empty_dir() {
    let mut manager = PluginManager::new();
    let temp_dir = std::env::temp_dir().join("plugin_test_empty");
    std::fs::create_dir_all(&temp_dir).unwrap();

    let result = manager.load_plugins_from_dir(&temp_dir);
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());

    std::fs::remove_dir(&temp_dir).unwrap();
}

#[test]
fn test_plugin_manager_unload_nonexistent() {
    let mut manager = PluginManager::new();
    let result = manager.unload_plugin("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_plugin_manager_reload_nonexistent() {
    let mut manager = PluginManager::new();
    let result = manager.reload_plugin("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_registry_plugin_names() {
    let registry = PluginRegistry::new();
    let names = registry.plugin_names();
    assert!(names.is_empty());
}

#[test]
fn test_registry_contains() {
    let registry = PluginRegistry::new();
    assert!(!registry.contains("test"));
}

#[test]
fn test_registry_get_by_name() {
    let registry = PluginRegistry::new();
    assert!(registry.get_by_name("test").is_none());
}

#[test]
fn test_shared_registry_creation() {
    let registry = plugin_system::new_shared_registry();
    let read_guard = registry.read().unwrap();
    assert!(read_guard.is_empty());
}

#[test]
fn test_plugin_context_creation() {
    let registry = plugin_system::new_shared_registry();
    let ctx = plugin_system::PluginContext::new(registry.clone());

    // Context should provide access to the registry
    let names = ctx.registry().plugin_names();
    assert!(names.is_empty());
}

#[test]
fn test_plugin_metadata_empty_authors() {
    let metadata = plugin_system::PluginMetadata {
        name: "test".to_string(),
        version: "0.1.0".to_string(),
        authors: vec![],
        dependencies: vec![],
    };

    assert!(metadata.authors.is_empty());
}

#[test]
fn test_plugin_metadata_many_authors() {
    let metadata = plugin_system::PluginMetadata {
        name: "test".to_string(),
        version: "0.1.0".to_string(),
        authors: (0..100).map(|i| format!("author_{}", i)).collect(),
        dependencies: vec![],
    };

    assert_eq!(metadata.authors.len(), 100);
}

#[test]
fn test_plugin_metadata_special_characters() {
    let metadata = plugin_system::PluginMetadata {
        name: "my-plugin_v2.0".to_string(),
        version: "2.0.0-beta.1+build.123".to_string(),
        authors: vec!["Author <email@example.com>".to_string()],
        dependencies: vec!["dep-1.0".to_string()],
    };

    assert_eq!(metadata.name, "my-plugin_v2.0");
    assert_eq!(metadata.version, "2.0.0-beta.1+build.123");
    assert_eq!(metadata.authors[0], "Author <email@example.com>");
    assert_eq!(metadata.dependencies[0], "dep-1.0");
}
