use plugin_interfaces::{Greet, GreetingService};
use plugin_system::{copy_cargo_plugin, library_path, PluginManager};
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

    // Demonstrate plugin interaction
    println!("\n=== Plugin Interaction ===");

    {
        let registry = manager.registry();
        let registry = registry.read().expect("registry lock poisoned");

        // Try to use the hello plugin
        if let Some(hello_arc) = registry.get_by_name("hello") {
            if let Ok(hello) = hello_arc.read() {
                if let Some(hello_plugin) = hello.downcast_ref::<plugin_types::HelloPlugin>() {
                    println!("Hello says: {}", hello_plugin.greet("Charlie"));
                }
            }
        }

        // Try to use the greeter plugin
        if let Some(greeter_arc) = registry.get_by_name("greeter") {
            if let Ok(greeter) = greeter_arc.read() {
                if let Some(greeter_plugin) =
                    greeter.downcast_ref::<plugin_types::GreeterPlugin>()
                {
                    println!("Greeter says: {}", greeter_plugin.greet("Alice"));
                    println!("Greeter says: {}", greeter_plugin.greet("Bob"));
                    println!("Total greetings: {}", greeter_plugin.count_greetings());
                }
            }
        }
    }

    // Demonstrate loader usage
    println!("\n=== Loader Demo ===");
    println!("You can also load plugins using specific loaders:");
    println!("  let loader = FileLoader::new(\"{}\");", library_path(".", "my_plugin").display());
    println!("  manager.load_plugin_from_loader(&loader, \"plugin_name\");");

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
        match copy_cargo_plugin(&target_dir, &plugins_dir, name)? {
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
