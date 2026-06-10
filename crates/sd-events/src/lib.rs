use sd_types::{ActionId, DeviceId, ProfileId, PluginResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

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
        let subscribers = self.subscribers.clone();

        tokio::spawn(async move {
            let mut subs = subscribers.write().await;
            subs.entry(key).or_default().push(cb);
        });
    }

    pub fn subscribe_all<F>(&self, callback: F)
    where
        F: Fn(&StreamEvent) + Send + Sync + 'static,
    {
        let cb = Arc::new(callback);
        let subscribers = self.subscribers.clone();

        tokio::spawn(async move {
            let mut subs = subscribers.write().await;
            subs.entry("*".to_string()).or_default().push(cb);
        });
    }

    pub async fn run(&self) {
        let mut rx = self.tx.subscribe();
        let subscribers = self.subscribers.clone();

        loop {
            match rx.recv().await {
                Ok(event) => {
                    let subs = subscribers.read().await;

                    if let Some(callbacks) = subs.get("*") {
                        for cb in callbacks {
                            cb(&event);
                        }
                    }

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

                    if let Some(callbacks) = subs.get(event_type) {
                        for cb in callbacks {
                            cb(&event);
                        }
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
