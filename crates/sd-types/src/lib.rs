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
        Self { value: PluginValue::String(s.into()) }
    }

    pub fn int(i: i64) -> Self {
        Self { value: PluginValue::Int(i) }
    }

    pub fn float(f: f64) -> Self {
        Self { value: PluginValue::Float(f) }
    }

    pub fn bool(b: bool) -> Self {
        Self { value: PluginValue::Bool(b) }
    }

    pub fn null() -> Self {
        Self { value: PluginValue::Null }
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
            pages: vec![Page { buttons: vec![ButtonBinding::empty(); 15] }],
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
