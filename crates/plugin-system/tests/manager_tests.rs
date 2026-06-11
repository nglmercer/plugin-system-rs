use plugin_system::PluginManager;
use std::path::PathBuf;
use tempfile::TempDir;

fn real_plugin_timer_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../..");
    plugin_system::platform::library_path(workspace_root.join("target/debug"), "plugin_timer")
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

#[test]
fn test_load_plugin_real_plugin() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!(
            "Skipping test_load_plugin_real_plugin: timer plugin not found at {}",
            path.display()
        );
        return;
    }
    let name = manager.load_plugin(&path).unwrap();
    assert_eq!(name, "timer");
    assert!(manager.is_loaded("timer"));
    assert!(manager.plugin_names().contains(&"timer".to_string()));
    let meta = manager.plugin_metadata("timer");
    assert!(meta.is_some());
    let meta = meta.unwrap();
    assert_eq!(meta.name, "timer");
    assert_eq!(meta.version, "0.1.0");
}

#[test]
fn test_unload_plugin() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping test_unload_plugin: timer plugin not found");
        return;
    }
    let name = manager.load_plugin(&path).unwrap();
    assert!(manager.is_loaded(&name));
    manager.unload_plugin(&name).unwrap();
    assert!(!manager.is_loaded(&name));
    assert!(manager.plugin_metadata(&name).is_none());
}

#[test]
fn test_with_plugin() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping test_with_plugin: timer plugin not found");
        return;
    }
    manager.load_plugin(&path).unwrap();
    let result = manager.with_plugin("timer", |plugin| plugin.plugin_type_name());
    assert!(result.is_ok());
    let type_name = result.unwrap();
    assert!(type_name.contains("TimerPlugin"));
}

#[test]
fn test_with_plugin_mut() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping test_with_plugin_mut: timer plugin not found");
        return;
    }
    manager.load_plugin(&path).unwrap();
    let result = manager.with_plugin_mut("timer", |plugin| {
        plugin.on_load(&plugin_system::PluginContext::new(
            manager.registry().clone(),
        ));
    });
    assert!(result.is_ok());
}

#[test]
fn test_get_all_plugin_info() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping test_get_all_plugin_info: timer plugin not found");
        return;
    }
    manager.load_plugin(&path).unwrap();
    let infos = manager.get_all_plugin_info();
    assert_eq!(infos.len(), 1);
    let info = &infos[0];
    assert_eq!(info.name, "timer");
    assert_eq!(info.version, "0.1.0");
}

#[test]
fn test_plugins_with_interface() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping test_plugins_with_interface: timer plugin not found");
        return;
    }
    manager.load_plugin(&path).unwrap();
    let names = manager.plugins_with_interface("Timer");
    assert_eq!(names, vec!["timer"]);
    let none = manager.plugins_with_interface("NonExistent");
    assert!(none.is_empty());
}

#[test]
fn test_reload_plugin() {
    let mut manager = PluginManager::new();
    let path = real_plugin_timer_path();
    if !path.exists() {
        eprintln!("Skipping test_reload_plugin: timer plugin not found");
        return;
    }
    let original_name = manager.load_plugin(&path).unwrap();
    assert!(manager.is_loaded(&original_name));
    manager.reload_plugin(&original_name).unwrap();
    assert!(manager.is_loaded(&original_name));
    let meta = manager.plugin_metadata(&original_name);
    assert!(meta.is_some());
    assert_eq!(meta.unwrap().name, "timer");
}

#[test]
fn test_load_plugin_missing_dependency() {
    let mut manager = PluginManager::new();
    let path = std::path::PathBuf::from(
        "/tmp/plugin-fake-missing-dep/target/debug/libplugin_fake_missing_dep.so",
    );
    if !path.exists() {
        eprintln!("Skipping test_load_plugin_missing_dependency: mock plugin not found");
        return;
    }
    let result = manager.load_plugin(path);
    assert!(result.is_err(), "Expected error but got Ok");
    let err = result.unwrap_err();
    eprintln!("Got error: {:?}", err);
    assert!(
        matches!(
            err,
            plugin_system::PluginError::MissingDependency { .. }
                | plugin_system::PluginError::SymbolNotFound { .. }
        ),
        "Expected MissingDependency or SymbolNotFound, got: {:?}",
        err
    );
}
