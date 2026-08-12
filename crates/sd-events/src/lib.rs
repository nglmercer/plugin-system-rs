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

    /// Subscribe to the raw broadcast stream.
    ///
    /// Prefer this over [`EventBus::subscribe_all`] for anything with a
    /// lifetime shorter than the process. A callback registered through
    /// `subscribe_all` lives in a map that is never pruned and is invoked by
    /// the single [`EventBus::run`] loop, so a per-connection consumer both
    /// leaks and depends on that loop staying alive. A receiver unregisters
    /// itself when dropped and is fed by the broadcast channel directly, which
    /// keeps a WebSocket client working even if `run()` is not spawned.
    pub fn subscribe_channel(&self) -> broadcast::Receiver<StreamEvent> {
        self.tx.subscribe()
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

    /// Dispatch loop for callbacks registered via [`EventBus::subscribe`] and
    /// [`EventBus::subscribe_all`].
    ///
    /// One panicking subscriber must not take the loop down with it: this is
    /// the only dispatcher those callbacks have, and losing it silently stops
    /// every subsequent delivery. Each callback therefore runs inside
    /// `catch_unwind` and a panic is logged and stepped over.
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
                        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cb(&event)))
                            .is_err()
                        {
                            log::error!(
                                "event subscriber panicked handling '{event_type}'; \
                                 continuing with the remaining subscribers"
                            );
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    log::warn!("event dispatcher lagged; {skipped} event(s) dropped");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    // Only reachable once every sender is gone, i.e. the bus
                    // itself was dropped. Say so rather than ending silently:
                    // a stopped dispatcher means no subscriber sees anything
                    // again, and that is worth a line in the log.
                    log::error!("event bus closed; callback dispatch has stopped");
                    break;
                }
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn event_bus_new() {
        let bus = EventBus::new();
        let _ = bus.tx.send(StreamEvent::PluginLoaded {
            plugin: "test".to_string(),
        });
    }

    #[test]
    fn event_bus_subscribe_receives_events() {
        use std::sync::Arc;

        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        bus.subscribe("button_pressed", move |_event| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let subs = bus.subscribers.read().unwrap();
        assert!(subs.contains_key("button_pressed"));
        assert_eq!(subs["button_pressed"].len(), 1);
    }

    /// One bad subscriber used to end the only dispatch loop in the process,
    /// which silently stopped every WebSocket client. The loop must survive it.
    #[tokio::test]
    async fn a_panicking_subscriber_does_not_stop_dispatch() {
        use std::sync::Arc;

        let bus = Arc::new(EventBus::new());
        let count = Arc::new(AtomicUsize::new(0));

        bus.subscribe_all(|_event| panic!("subscriber is broken"));
        let count_clone = count.clone();
        bus.subscribe_all(move |_event| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        let bus_clone = bus.clone();
        let dispatcher = tokio::spawn(async move { bus_clone.run().await });

        // `run()` subscribes when it starts, and a broadcast channel delivers
        // nothing sent before that. Wait for the subscription rather than
        // racing the spawn.
        while bus.tx.receiver_count() == 0 {
            tokio::task::yield_now().await;
        }

        for _ in 0..2 {
            bus.emit(StreamEvent::PluginLoaded {
                plugin: "test".to_string(),
            });
        }

        // Give the loop a chance to drain both events before asserting.
        for _ in 0..100 {
            if count.load(Ordering::SeqCst) == 2 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        assert_eq!(
            count.load(Ordering::SeqCst),
            2,
            "the healthy subscriber must still receive both events"
        );
        dispatcher.abort();
    }

    /// A channel subscription is the right shape for a per-connection consumer:
    /// it needs no dispatch loop and unregisters itself on drop.
    #[tokio::test]
    async fn channel_subscribers_receive_events_without_the_dispatch_loop() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe_channel();

        bus.emit(StreamEvent::PluginLoaded {
            plugin: "test".to_string(),
        });

        let event = rx.recv().await.expect("event should arrive");
        assert!(matches!(event, StreamEvent::PluginLoaded { .. }));
    }

    #[test]
    fn stream_event_variants_serialize() {
        let events = vec![
            StreamEvent::PluginLoaded {
                plugin: "test".to_string(),
            },
            StreamEvent::PluginUnloaded {
                plugin: "test".to_string(),
            },
            StreamEvent::DeviceConnected {
                device: DeviceId("d".to_string()),
            },
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            assert!(!json.is_empty());
        }
    }
}
