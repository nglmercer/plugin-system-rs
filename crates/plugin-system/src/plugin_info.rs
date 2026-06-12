use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub dependencies: Vec<String>,
    pub public_methods: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum PluginResult {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<String>),
    Map(HashMap<String, String>),
    Null,
}

impl PluginResult {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            PluginResult::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            PluginResult::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            PluginResult::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PluginResult::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&Vec<String>> {
        match self {
            PluginResult::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<String, String>> {
        match self {
            PluginResult::Map(m) => Some(m),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, PluginResult::Null)
    }
}

impl std::fmt::Display for PluginResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginResult::String(s) => write!(f, "{}", s),
            PluginResult::Int(i) => write!(f, "{}", i),
            PluginResult::Float(fl) => write!(f, "{}", fl),
            PluginResult::Bool(b) => write!(f, "{}", b),
            PluginResult::List(l) => write!(f, "{:?}", l),
            PluginResult::Map(m) => write!(f, "{:?}", m),
            PluginResult::Null => write!(f, "null"),
        }
    }
}

impl From<String> for PluginResult {
    fn from(s: String) -> Self {
        PluginResult::String(s)
    }
}

impl From<&str> for PluginResult {
    fn from(s: &str) -> Self {
        PluginResult::String(s.to_string())
    }
}

impl From<i64> for PluginResult {
    fn from(i: i64) -> Self {
        PluginResult::Int(i)
    }
}

impl From<f64> for PluginResult {
    fn from(f: f64) -> Self {
        PluginResult::Float(f)
    }
}

impl From<bool> for PluginResult {
    fn from(b: bool) -> Self {
        PluginResult::Bool(b)
    }
}

impl From<Vec<String>> for PluginResult {
    fn from(v: Vec<String>) -> Self {
        PluginResult::List(v)
    }
}

impl<T: Into<PluginResult>> From<Option<T>> for PluginResult {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),
            None => PluginResult::Null,
        }
    }
}
