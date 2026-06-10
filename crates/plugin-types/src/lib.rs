use plugin_interfaces::{Calculator, DataProvider, Greet, GreetingService, Logger};
use std::collections::HashMap;

pub struct HelloPlugin {
    greeting: String,
    data: HashMap<String, String>,
}

impl Default for HelloPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl HelloPlugin {
    pub fn new() -> Self {
        Self {
            greeting: "Hello".to_string(),
            data: HashMap::new(),
        }
    }

    pub fn set_greeting(&mut self, greeting: String) {
        self.greeting = greeting;
    }

    pub fn get_greeting(&self) -> &str {
        &self.greeting
    }

    pub fn add_data(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }

    pub fn get_data_value(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
}

impl Greet for HelloPlugin {
    fn greet(&self, name: &str) -> String {
        format!("{}, {}!", self.greeting, name)
    }

    fn get_name(&self) -> &str {
        "HelloPlugin"
    }
}

impl DataProvider for HelloPlugin {
    fn get_data(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    fn set_data(&mut self, key: &str, value: String) {
        self.data.insert(key.to_string(), value);
    }

    fn list_keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }
}

pub struct GreeterPlugin {
    greeting_count: u64,
    last_greeting: String,
    logs: Vec<String>,
}

impl Default for GreeterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl GreeterPlugin {
    pub fn new() -> Self {
        Self {
            greeting_count: 0,
            last_greeting: String::new(),
            logs: Vec::new(),
        }
    }

    pub fn get_count(&self) -> u64 {
        self.greeting_count
    }

    pub fn get_last_greeting(&self) -> &str {
        &self.last_greeting
    }
}

impl GreetingService for GreeterPlugin {
    fn greet(&self, name: &str) -> String {
        format!("Welcome, {}! (via GreeterPlugin)", name)
    }

    fn count_greetings(&self) -> u64 {
        self.greeting_count
    }
}

impl Calculator for GreeterPlugin {
    fn add(&self, a: f64, b: f64) -> f64 {
        a + b
    }

    fn subtract(&self, a: f64, b: f64) -> f64 {
        a - b
    }

    fn multiply(&self, a: f64, b: f64) -> f64 {
        a * b
    }

    fn divide(&self, a: f64, b: f64) -> Option<f64> {
        if b == 0.0 {
            None
        } else {
            Some(a / b)
        }
    }
}

impl Logger for GreeterPlugin {
    fn log_info(&self, message: &str) {
        log::info!("[GreeterPlugin] {}", message);
    }

    fn log_error(&self, message: &str) {
        log::error!("[GreeterPlugin] {}", message);
    }

    fn get_logs(&self) -> Vec<String> {
        self.logs.clone()
    }
}
