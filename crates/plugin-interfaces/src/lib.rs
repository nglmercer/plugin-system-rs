use plugin_macros::plugin_interface;

#[plugin_interface]
pub trait Greet {
    fn greet(&self, name: &str) -> String;
    fn get_name(&self) -> &str;
}

#[plugin_interface]
pub trait KeySimulator: Send + Sync {
    fn simulate_keys(&self, keys: &[String]) -> Result<(), String>;
}
