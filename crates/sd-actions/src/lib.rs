use sd_events::EventBus;
use sd_types::{ActionId, DeviceId, PluginResult, ProfileId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ActionContext {
    pub device_id: DeviceId,
    pub button_index: usize,
    pub profile_id: ProfileId,
    pub state: Arc<RwLock<HashMap<String, PluginResult>>>,
    pub events: Arc<EventBus>,
}

pub trait Action: Send + Sync + 'static {
    fn action_id(&self) -> ActionId;
    fn action_name(&self) -> &str;
    fn category(&self) -> &str;
    fn execute(&self, ctx: &ActionContext) -> PluginResult;
    fn on_tick(&self, _ctx: &ActionContext) {}
}

// Built-in: Hotkey Action
#[derive(Debug)]
pub struct HotkeyAction {
    pub keys: String,
}

impl HotkeyAction {
    pub fn new(keys: impl Into<String>) -> Self {
        Self { keys: keys.into() }
    }
}

impl Action for HotkeyAction {
    fn action_id(&self) -> ActionId {
        ActionId("hotkey".to_string())
    }

    fn action_name(&self) -> &str {
        "Send Hotkey"
    }

    fn category(&self) -> &str {
        "System"
    }

    fn execute(&self, ctx: &ActionContext) -> PluginResult {
        println!(
            "[HotkeyAction] Sending keys: {} (device: {:?}, button: {})",
            self.keys, ctx.device_id, ctx.button_index
        );

        // In real implementation, this would use enigo or similar
        // to actually send keyboard events
        ctx.events.emit(sd_events::StreamEvent::ActionExecuted {
            action: self.action_id(),
            result: PluginResult::string(format!("Sent: {}", self.keys)),
        });

        PluginResult::string(format!("Sent: {}", self.keys))
    }
}

// Built-in: Text Action
#[derive(Debug)]
pub struct TextAction {
    pub text: String,
}

impl TextAction {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl Action for TextAction {
    fn action_id(&self) -> ActionId {
        ActionId("text".to_string())
    }

    fn action_name(&self) -> &str {
        "Type Text"
    }

    fn category(&self) -> &str {
        "System"
    }

    fn execute(&self, ctx: &ActionContext) -> PluginResult {
        println!(
            "[TextAction] Typing: {} (device: {:?}, button: {})",
            self.text, ctx.device_id, ctx.button_index
        );

        ctx.events.emit(sd_events::StreamEvent::ActionExecuted {
            action: self.action_id(),
            result: PluginResult::string(format!("Typed: {}", self.text)),
        });

        PluginResult::string(format!("Typed: {}", self.text))
    }
}

// Built-in: Open URL Action
#[derive(Debug)]
pub struct OpenUrlAction {
    pub url: String,
}

impl OpenUrlAction {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

impl Action for OpenUrlAction {
    fn action_id(&self) -> ActionId {
        ActionId("open_url".to_string())
    }

    fn action_name(&self) -> &str {
        "Open URL"
    }

    fn category(&self) -> &str {
        "System"
    }

    fn execute(&self, ctx: &ActionContext) -> PluginResult {
        log::info!("[OpenUrlAction] Opening: {}", self.url);

        if let Err(e) = open::that(&self.url) {
            log::error!("[OpenUrlAction] Failed to open URL: {}", e);
            return PluginResult::string(format!("Failed: {}", e));
        }

        ctx.events.emit(sd_events::StreamEvent::ActionExecuted {
            action: self.action_id(),
            result: PluginResult::string(format!("Opened: {}", self.url)),
        });

        PluginResult::string(format!("Opened: {}", self.url))
    }
}

// Action Registry
pub struct ActionRegistry {
    actions: HashMap<String, Arc<dyn Action>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            actions: HashMap::new(),
        }
    }

    pub fn register(&mut self, action: Arc<dyn Action>) {
        let id = action.action_id().0.clone();
        self.actions.insert(id, action);
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn Action>> {
        self.actions.get(id)
    }

    pub fn list(&self) -> Vec<&Arc<dyn Action>> {
        self.actions.values().collect()
    }

    pub fn list_by_category(&self) -> HashMap<String, Vec<&Arc<dyn Action>>> {
        let mut by_category: HashMap<String, Vec<&Arc<dyn Action>>> = HashMap::new();
        for action in self.actions.values() {
            by_category
                .entry(action.category().to_string())
                .or_default()
                .push(action);
        }
        by_category
    }

    pub fn find_by_name(&self, name: &str) -> Option<&Arc<dyn Action>> {
        self.actions.values().find(|a| a.action_name() == name)
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_ctx() -> ActionContext {
        ActionContext {
            device_id: DeviceId("test-device".to_string()),
            button_index: 0,
            profile_id: ProfileId(uuid::Uuid::new_v4()),
            state: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            events: Arc::new(EventBus::new()),
        }
    }

    #[test]
    fn hotkey_action_metadata() {
        let action = HotkeyAction::new("Ctrl+C");
        assert_eq!(action.action_id().0, "hotkey");
        assert_eq!(action.action_name(), "Send Hotkey");
        assert_eq!(action.category(), "System");
    }

    #[test]
    fn text_action_execute() {
        let action = TextAction::new("hello");
        let result = action.execute(&test_ctx());
        assert_eq!(result.as_string(), Some("Typed: hello"));
    }

    #[test]
    fn open_url_action_metadata() {
        let action = OpenUrlAction::new("https://example.com");
        assert_eq!(action.action_id().0, "open_url");
        assert_eq!(action.action_name(), "Open URL");
    }

    #[test]
    fn action_registry_register_and_get() {
        let mut registry = ActionRegistry::new();
        let action = Arc::new(HotkeyAction::new("A"));
        registry.register(action.clone());
        assert!(registry.get("hotkey").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn action_registry_list_by_category() {
        let mut registry = ActionRegistry::new();
        registry.register(Arc::new(HotkeyAction::new("A")));
        registry.register(Arc::new(TextAction::new("B")));
        let by_cat = registry.list_by_category();
        assert!(by_cat.contains_key("System"));
        assert_eq!(by_cat["System"].len(), 2);
    }
}
