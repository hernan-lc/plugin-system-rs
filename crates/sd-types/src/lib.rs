use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ActionId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ProfileId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DeviceId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub value: PluginValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<PluginValue>),
    Map(HashMap<String, PluginValue>),
    Null,
}

impl PluginResult {
    pub fn string(s: impl Into<String>) -> Self {
        Self {
            value: PluginValue::String(s.into()),
        }
    }

    pub fn int(i: i64) -> Self {
        Self {
            value: PluginValue::Int(i),
        }
    }

    pub fn float(f: f64) -> Self {
        Self {
            value: PluginValue::Float(f),
        }
    }

    pub fn bool(b: bool) -> Self {
        Self {
            value: PluginValue::Bool(b),
        }
    }

    pub fn null() -> Self {
        Self {
            value: PluginValue::Null,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match &self.value {
            PluginValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match &self.value {
            PluginValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match &self.value {
            PluginValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            PluginValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

impl std::fmt::Display for PluginResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            PluginValue::String(s) => write!(f, "{}", s),
            PluginValue::Int(i) => write!(f, "{}", i),
            PluginValue::Float(fl) => write!(f, "{}", fl),
            PluginValue::Bool(b) => write!(f, "{}", b),
            PluginValue::Null => write!(f, "null"),
            PluginValue::List(l) => write!(f, "{:?}", l),
            PluginValue::Map(m) => write!(f, "{:?}", m),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: ProfileId,
    pub name: String,
    pub pages: Vec<Page>,
}

impl Profile {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: ProfileId(Uuid::new_v4()),
            name: name.into(),
            pages: vec![Page {
                buttons: vec![ButtonBinding::empty(); 15],
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub buttons: Vec<ButtonBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonBinding {
    pub action_id: Option<ActionId>,
    pub settings: HashMap<String, PluginResult>,
    pub label: String,
    pub icon: String,
}

impl ButtonBinding {
    pub fn empty() -> Self {
        Self {
            action_id: None,
            settings: HashMap::new(),
            label: String::new(),
            icon: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub key_count: usize,
    pub is_virtual: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_result_constructors() {
        assert_eq!(PluginResult::string("hello").as_string(), Some("hello"));
        assert_eq!(PluginResult::int(42).as_int(), Some(42));
        assert_eq!(PluginResult::float(3.14).as_float(), Some(3.14));
        assert_eq!(PluginResult::bool(true).as_bool(), Some(true));
        assert_eq!(PluginResult::null().as_string(), None);
    }

    #[test]
    fn profile_new_creates_with_one_page() {
        let profile = Profile::new("Test");
        assert_eq!(profile.name, "Test");
        assert_eq!(profile.pages.len(), 1);
        assert_eq!(profile.pages[0].buttons.len(), 15);
    }

    #[test]
    fn button_binding_empty() {
        let b = ButtonBinding::empty();
        assert!(b.action_id.is_none());
        assert!(b.settings.is_empty());
        assert!(b.label.is_empty());
        assert!(b.icon.is_empty());
    }

    #[test]
    fn plugin_result_display() {
        assert_eq!(PluginResult::string("hi").to_string(), "hi");
        assert_eq!(PluginResult::int(5).to_string(), "5");
        assert_eq!(PluginResult::null().to_string(), "null");
    }
}
