use plugin_interfaces::{Greet, GreetingService};
use plugin_system::PluginManager;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Plugin System Host Application ===\n");

    let mut manager = PluginManager::new();

    let plugin_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./plugins".to_string());

    println!("Scanning for plugins in: {}", plugin_dir);

    let loaded = manager.load_plugins_from_dir(&plugin_dir)?;

    if loaded.is_empty() {
        println!("No plugins found. Place .so/.dll/.dylib files in the plugins directory.");
        println!("Building example plugins...\n");
        build_example_plugins()?;
        let _ = manager.load_plugins_from_dir(&plugin_dir)?;
    }

    println!("\n=== Loaded Plugins ===");
    for name in manager.plugin_names() {
        if let Some(meta) = manager.plugin_metadata(&name) {
            println!(
                "  - {} v{} (authors: {}, deps: {:?})",
                meta.name,
                meta.version,
                meta.authors.join(", "),
                meta.dependencies
            );
        }
    }

    println!("\n=== Dynamic Interface Discovery (with_plugin + downcast) ===");

    println!("\n--- Greet interface on hello plugin ---");
    let result = manager.with_plugin("hello", |plugin| {
        let greet: &dyn Greet = plugin
            .downcast_ref::<plugin_types::HelloPlugin>()
            .expect("hello must implement Greet");
        greet.greet("World")
    })?;
    println!("  hello.greet(\"World\") = {}", result);

    let result = manager.with_plugin("hello", |plugin| {
        let greet: &dyn Greet = plugin
            .downcast_ref::<plugin_types::HelloPlugin>()
            .expect("hello must implement Greet");
        greet.get_name().to_string()
    })?;
    println!("  hello.get_name() = {}", result);

    println!("\n--- GreetingService interface on greeter plugin ---");
    let result = manager.with_plugin("greeter", |plugin| {
        let svc: &dyn GreetingService = plugin
            .downcast_ref::<plugin_types::GreeterPlugin>()
            .expect("greeter must implement GreetingService");
        svc.greet("Alice")
    })?;
    println!("  greeter.greet(\"Alice\") = {}", result);

    let result = manager.with_plugin("greeter", |plugin| {
        let svc: &dyn GreetingService = plugin
            .downcast_ref::<plugin_types::GreeterPlugin>()
            .expect("greeter must implement GreetingService");
        svc.greet("Bob")
    })?;
    println!("  greeter.greet(\"Bob\") = {}", result);

    let count = manager.with_plugin("greeter", |plugin| {
        let svc: &dyn GreetingService = plugin
            .downcast_ref::<plugin_types::GreeterPlugin>()
            .expect("greeter must implement GreetingService");
        svc.count_greetings()
    })?;
    println!("  greeter.count_greetings() = {}", count);

    println!("\n=== Plugin Interaction (Commands) ===");

    println!("\n--- Hello Plugin Help ---");
    match manager.call_plugin("hello", "help") {
        Ok(help) => println!("{}", help),
        Err(e) => println!("Error: {}", e),
    }

    println!("\n--- Hello Plugin Greet ---");
    match manager.call_plugin("hello", "greet Rust Developer") {
        Ok(result) => println!("{}", result),
        Err(e) => println!("Error: {}", e),
    }

    println!("\n--- Greeter Plugin Info ---");
    match manager.call_plugin("greeter", "info") {
        Ok(result) => println!("{}", result),
        Err(e) => println!("Error: {}", e),
    }

    println!("\n=== Call All Plugins: info ===");
    let results = manager.call_plugins("info");
    for (name, result) in &results {
        println!("\n{}:", name);
        println!("{}", result);
    }

    println!("\n=== Plugin System Ready ===");

    Ok(())
}

fn build_example_plugins() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = Path::new(manifest_dir)
        .parent()
        .unwrap_or(Path::new("."))
        .parent()
        .unwrap_or(Path::new("."));

    println!("Building example plugins from workspace...");

    let status = std::process::Command::new("cargo")
        .args(["build", "-p", "plugin-hello", "--release"])
        .current_dir(workspace_root)
        .status()?;

    if !status.success() {
        return Err("Failed to build plugin-hello".into());
    }

    let status = std::process::Command::new("cargo")
        .args(["build", "-p", "plugin-greeter", "--release"])
        .current_dir(workspace_root)
        .status()?;

    if !status.success() {
        return Err("Failed to build plugin-greeter".into());
    }

    let plugins_dir = workspace_root.join("plugins");
    std::fs::create_dir_all(&plugins_dir)?;

    let target_dir = workspace_root.join("target").join("release");

    for name in &["plugin_hello", "plugin_greeter"] {
        match plugin_system::copy_cargo_plugin(&target_dir, &plugins_dir, name)? {
            Some(dest) => println!("  Copied {}", dest.display()),
            None => println!("  Warning: {} not found in {}", name, target_dir.display()),
        }
    }

    println!(
        "Example plugins built and copied to {}",
        plugins_dir.display()
    );
    Ok(())
}
