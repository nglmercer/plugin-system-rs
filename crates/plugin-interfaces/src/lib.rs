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
