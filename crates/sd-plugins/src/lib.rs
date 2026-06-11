use plugin_system::PluginManager;
use sd_actions::ActionRegistry;
use sd_events::EventBus;
use sd_types::ActionId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SdPluginManager {
    plugin_manager: Arc<RwLock<PluginManager>>,
    action_registry: Arc<RwLock<ActionRegistry>>,
    events: Arc<EventBus>,
    plugin_actions: Arc<RwLock<HashMap<String, Vec<ActionId>>>>,
}

impl SdPluginManager {
    pub fn new(events: Arc<EventBus>, action_registry: Arc<RwLock<ActionRegistry>>) -> Self {
        Self {
            plugin_manager: Arc::new(RwLock::new(PluginManager::new())),
            action_registry,
            events,
            plugin_actions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn load_plugins_from_dir(&self, dir: &str) -> Result<Vec<String>, String> {
        let mut manager = self.plugin_manager.write().await;
        manager
            .load_plugins_from_dir(dir)
            .map_err(|e| e.to_string())
    }

    pub async fn list_plugins(&self) -> Vec<String> {
        let manager = self.plugin_manager.read().await;
        manager.plugin_names()
    }

    pub async fn reload_plugins(&self) -> Result<(), String> {
        let mut manager = self.plugin_manager.write().await;
        for name in manager.plugin_names() {
            manager.reload_plugin(&name).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn plugin_manager(&self) -> Arc<RwLock<PluginManager>> {
        self.plugin_manager.clone()
    }

    pub fn events(&self) -> &Arc<EventBus> {
        &self.events
    }

    pub fn action_registry(&self) -> &Arc<RwLock<ActionRegistry>> {
        &self.action_registry
    }

    pub async fn plugin_actions(&self) -> HashMap<String, Vec<ActionId>> {
        self.plugin_actions.read().await.clone()
    }
}
