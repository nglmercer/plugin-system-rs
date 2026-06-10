use plugin_macros::plugin_interface;

#[plugin_interface]
pub trait Greet {
    fn greet(&self, name: &str) -> String;
    fn get_name(&self) -> &str;
}

#[plugin_interface]
pub trait GreetingService {
    fn greet(&self, name: &str) -> String;
    fn count_greetings(&self) -> u64;
}

#[plugin_interface]
pub trait DataProvider {
    fn get_data(&self, key: &str) -> Option<String>;
    fn set_data(&mut self, key: &str, value: String);
    fn list_keys(&self) -> Vec<String>;
}

#[plugin_interface]
pub trait Calculator {
    fn add(&self, a: f64, b: f64) -> f64;
    fn subtract(&self, a: f64, b: f64) -> f64;
    fn multiply(&self, a: f64, b: f64) -> f64;
    fn divide(&self, a: f64, b: f64) -> Option<f64>;
}

#[plugin_interface]
pub trait Logger {
    fn log_info(&self, message: &str);
    fn log_error(&self, message: &str);
    fn get_logs(&self) -> Vec<String>;
}
