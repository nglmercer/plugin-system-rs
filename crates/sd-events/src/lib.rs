use sd_types::{ActionId, DeviceId, PluginResult, ProfileId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    ButtonPressed {
        device: DeviceId,
        index: usize,
        profile: ProfileId,
    },
    ButtonReleased {
        device: DeviceId,
        index: usize,
    },
    ProfileChanged {
        profile: ProfileId,
    },
    ActionExecuted {
        action: ActionId,
        result: PluginResult,
    },
    PluginLoaded {
        plugin: String,
    },
    PluginUnloaded {
        plugin: String,
    },
    DeviceConnected {
        device: DeviceId,
    },
    DeviceDisconnected {
        device: DeviceId,
    },
}

type EventCallback = Arc<dyn Fn(&StreamEvent) + Send + Sync>;

pub struct EventBus {
    tx: broadcast::Sender<StreamEvent>,
    subscribers: Arc<RwLock<HashMap<String, Vec<EventCallback>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            tx,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn emit(&self, event: StreamEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe<F>(&self, event_type: &str, callback: F)
    where
        F: Fn(&StreamEvent) + Send + Sync + 'static,
    {
        let cb = Arc::new(callback);
        let key = event_type.to_string();
        let mut subs = self.subscribers.write().unwrap();
        subs.entry(key).or_default().push(cb);
    }

    pub fn subscribe_all<F>(&self, callback: F)
    where
        F: Fn(&StreamEvent) + Send + Sync + 'static,
    {
        let cb = Arc::new(callback);
        let mut subs = self.subscribers.write().unwrap();
        subs.entry("*".to_string()).or_default().push(cb);
    }

    pub async fn run(&self) {
        let mut rx = self.tx.subscribe();

        loop {
            match rx.recv().await {
                Ok(event) => {
                    let event_type = match &event {
                        StreamEvent::ButtonPressed { .. } => "button_pressed",
                        StreamEvent::ButtonReleased { .. } => "button_released",
                        StreamEvent::ProfileChanged { .. } => "profile_changed",
                        StreamEvent::ActionExecuted { .. } => "action_executed",
                        StreamEvent::PluginLoaded { .. } => "plugin_loaded",
                        StreamEvent::PluginUnloaded { .. } => "plugin_unloaded",
                        StreamEvent::DeviceConnected { .. } => "device_connected",
                        StreamEvent::DeviceDisconnected { .. } => "device_disconnected",
                    };

                    let subs = self.subscribers.read().unwrap();
                    let mut all_cbs: Vec<EventCallback> = Vec::new();
                    if let Some(cbs) = subs.get("*") {
                        all_cbs.extend(cbs.iter().cloned());
                    }
                    if let Some(cbs) = subs.get(event_type) {
                        all_cbs.extend(cbs.iter().cloned());
                    }
                    drop(subs);

                    for cb in all_cbs {
                        cb(&event);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
