use plugin_system::traits::{Plugin, PluginMetadata};
use plugin_system::PluginRegistry;

#[derive(Default)]
struct DummyPlugin {
    name: &'static str,
    interfaces: Vec<&'static str>,
}

impl Plugin for DummyPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::traits::PluginMetadata {
            name: self.name.to_string(),
            version: "0.1.0".to_string(),
            authors: vec!["Test".to_string()],
            dependencies: vec![],
        }
    }

    fn on_load(&mut self, _ctx: &plugin_system::PluginContext) {}
    fn on_unload(&mut self) {}
    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
    fn interface_ids(&self) -> Vec<&'static str> {
        self.interfaces.clone()
    }
}

#[test]
fn test_registry_new_is_empty() {
    let registry = PluginRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(registry.plugin_names().is_empty());
}

#[test]
fn test_register_and_get_by_name() {
    let mut registry = PluginRegistry::new();
    let plugin = DummyPlugin {
        name: "alpha",
        interfaces: vec![],
    };
    registry.register(Box::new(plugin));
    assert!(!registry.is_empty());
    assert_eq!(registry.len(), 1);
    assert!(registry.contains("alpha"));
    assert!(registry.get_by_name("alpha").is_some());
    assert!(registry.get_by_name("missing").is_none());
}

#[test]
fn test_get_by_interface() {
    let mut registry = PluginRegistry::new();
    let plugin_a = DummyPlugin {
        name: "plugin_a",
        interfaces: vec!["Timer"],
    };
    let plugin_b = DummyPlugin {
        name: "plugin_b",
        interfaces: vec!["KeySimulator"],
    };
    registry.register(Box::new(plugin_a));
    registry.register(Box::new(plugin_b));
    assert_eq!(registry.get_by_interface("Timer"), vec!["plugin_a"]);
    assert_eq!(registry.get_by_interface("KeySimulator"), vec!["plugin_b"]);
    assert!(registry.get_by_interface("Missing").is_empty());
}

#[test]
fn test_unregister() {
    let mut registry = PluginRegistry::new();
    let plugin = DummyPlugin {
        name: "beta",
        interfaces: vec![],
    };
    registry.register(Box::new(plugin));
    assert!(registry.contains("beta"));
    let removed = registry.unregister("beta");
    assert!(removed.is_some());
    assert!(!registry.contains("beta"));
    assert!(registry.unregister("beta").is_none());
}

#[test]
fn test_iter_plugins() {
    let mut registry = PluginRegistry::new();
    let plugin_a = DummyPlugin {
        name: "p1",
        interfaces: vec![],
    };
    let plugin_b = DummyPlugin {
        name: "p2",
        interfaces: vec![],
    };
    registry.register(Box::new(plugin_a));
    registry.register(Box::new(plugin_b));
    let names: Vec<_> = registry
        .iter_plugins()
        .map(|(name, _)| name.clone())
        .collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"p1".to_string()));
    assert!(names.contains(&"p2".to_string()));
    assert_eq!(registry.iter_plugins().count(), 2);
}
