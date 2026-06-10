use plugin_system::PluginManager;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== Plugin System Host Application ===\n");

    let mut manager = PluginManager::new();

    // Determine plugin directory
    let plugin_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "./plugins".to_string());

    println!("Scanning for plugins in: {}", plugin_dir);

    // Load all plugins from the directory using FileLoader
    let loaded = manager.load_plugins_from_dir(&plugin_dir)?;

    if loaded.is_empty() {
        println!("No plugins found. Place .so/.dll/.dylib files in the plugins directory.");
        println!("Building example plugins...\n");
        build_example_plugins()?;

        // Reload plugins after building
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

    // Demonstrate plugin interaction via commands
    println!("\n=== Plugin Interaction (Commands) ===");

    // Get help from hello plugin
    println!("\n--- Hello Plugin Help ---");
    match manager.call_plugin("hello", "help") {
        Ok(help) => println!("{}", help),
        Err(e) => println!("Error: {}", e),
    }

    // Greet using hello plugin
    println!("\n--- Hello Plugin Greet ---");
    match manager.call_plugin("hello", "greet World") {
        Ok(result) => println!("{}", result),
        Err(e) => println!("Error: {}", e),
    }

    // Get greeting from hello plugin
    println!("\n--- Hello Plugin Get Greeting ---");
    match manager.call_plugin("hello", "get_greeting") {
        Ok(result) => println!("{}", result),
        Err(e) => println!("Error: {}", e),
    }

    // Set new greeting
    println!("\n--- Hello Plugin Set Greeting ---");
    match manager.call_plugin("hello", "set_greeting Hi") {
        Ok(result) => println!("{}", result),
        Err(e) => println!("Error: {}", e),
    }

    // Greet again with new greeting
    println!("\n--- Hello Plugin Greet Again ---");
    match manager.call_plugin("hello", "greet Rust Developer") {
        Ok(result) => println!("{}", result),
        Err(e) => println!("Error: {}", e),
    }

    // Get plugin info
    println!("\n--- Hello Plugin Info ---");
    match manager.call_plugin("hello", "info") {
        Ok(result) => println!("{}", result),
        Err(e) => println!("Error: {}", e),
    }

    // Test greeter plugin
    println!("\n--- Greeter Plugin Help ---");
    match manager.call_plugin("greeter", "help") {
        Ok(help) => println!("{}", help),
        Err(e) => println!("Error: {}", e),
    }

    // Greet multiple times
    println!("\n--- Greeter Plugin Greet Multiple Times ---");
    for name in &["Alice", "Bob", "Charlie"] {
        match manager.call_plugin("greeter", &format!("greet {}", name)) {
            Ok(result) => println!("{}", result),
            Err(e) => println!("Error: {}", e),
        }
    }

    // Get greeting count
    println!("\n--- Greeter Plugin Count ---");
    match manager.call_plugin("greeter", "count") {
        Ok(result) => println!("{}", result),
        Err(e) => println!("Error: {}", e),
    }

    // Get last greeting
    println!("\n--- Greeter Plugin Last ---");
    match manager.call_plugin("greeter", "last") {
        Ok(result) => println!("{}", result),
        Err(e) => println!("Error: {}", e),
    }

    // Call all plugins with same command
    println!("\n--- Call All Plugins: info ---");
    let results = manager.call_plugins("info");
    for (name, result) in &results {
        println!("\n{}:", name);
        println!("{}", result);
    }

    println!("\n=== Plugin System Ready ===");
    println!("Press Ctrl+C to exit.");

    // Keep the host alive
    std::thread::park();

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

    // Build plugin-hello
    let status = std::process::Command::new("cargo")
        .args(["build", "-p", "plugin-hello", "--release"])
        .current_dir(workspace_root)
        .status()?;

    if !status.success() {
        return Err("Failed to build plugin-hello".into());
    }

    // Build plugin-greeter
    let status = std::process::Command::new("cargo")
        .args(["build", "-p", "plugin-greeter", "--release"])
        .current_dir(workspace_root)
        .status()?;

    if !status.success() {
        return Err("Failed to build plugin-greeter".into());
    }

    // Copy built plugins to the plugins directory
    let plugins_dir = workspace_root.join("plugins");
    std::fs::create_dir_all(&plugins_dir)?;

    let target_dir = workspace_root.join("target").join("release");

    // Copy plugins using helper function
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
