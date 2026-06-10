/// Interface for plugins that provide greeting functionality.
///
/// Plugins implementing this trait can be downcast from the registry
/// by the host or other plugins to call these methods directly.
pub trait Greet {
    fn greet(&self, name: &str) -> String;
    fn get_name(&self) -> &str;
}

/// Interface for plugins that provide a greeting service.
///
/// Demonstrates a more complex plugin interface with state tracking.
pub trait GreetingService {
    fn greet(&self, name: &str) -> String;
    fn count_greetings(&self) -> u64;
}
