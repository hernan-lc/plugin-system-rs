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
