use plugin_interfaces::{Calculator, DataProvider, Greet};
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

    println!("\n=== Interface Registry ===");
    let interfaces = manager.list_all_interfaces();
    for (iface, providers) in &interfaces {
        println!("  Interface '{}': provided by {:?}", iface, providers);
    }

    println!("\n=== Phase 1: Type-Safe Interface Access via with_plugin ===");

    println!("\n--- Greet interface on hello plugin ---");
    let result = manager.with_plugin("hello", |plugin| {
        let greet: &dyn Greet = plugin
            .downcast_ref::<plugin_types::HelloPlugin>()
            .expect("hello must implement Greet");
        greet.greet("World")
    })?;
    println!("  hello.greet(\"World\") = {}", result);

    println!("\n--- DataProvider interface on hello plugin ---");
    manager.with_plugin_mut("hello", |plugin| {
        let data: &mut dyn DataProvider = plugin
            .downcast_mut::<plugin_types::HelloPlugin>()
            .expect("hello must implement DataProvider");
        data.set_data("key1", "value1".to_string());
        data.set_data("key2", "value2".to_string());
    })?;

    let keys = manager.with_plugin("hello", |plugin| {
        let data: &dyn DataProvider = plugin
            .downcast_ref::<plugin_types::HelloPlugin>()
            .expect("hello must implement DataProvider");
        data.list_keys()
    })?;
    println!("  hello data keys: {:?}", keys);

    let value = manager.with_plugin("hello", |plugin| {
        let data: &dyn DataProvider = plugin
            .downcast_ref::<plugin_types::HelloPlugin>()
            .expect("hello must implement DataProvider");
        data.get_data("key1")
    })?;
    println!("  hello data[\"key1\"] = {:?}", value);

    println!("\n--- Calculator interface on greeter plugin ---");
    let result = manager.with_plugin("greeter", |plugin| {
        let calc: &dyn Calculator = plugin
            .downcast_ref::<plugin_types::GreeterPlugin>()
            .expect("greeter must implement Calculator");
        calc.add(10.0, 5.0)
    })?;
    println!("  greeter.add(10.0, 5.0) = {}", result);

    let result = manager.with_plugin("greeter", |plugin| {
        let calc: &dyn Calculator = plugin
            .downcast_ref::<plugin_types::GreeterPlugin>()
            .expect("greeter must implement Calculator");
        calc.divide(10.0, 0.0)
    })?;
    println!("  greeter.divide(10.0, 0.0) = {:?}", result);

    println!("\n=== Phase 2: Plugin-to-Plugin Direct Access ===");

    println!("\n--- Hello plugin calls Greeter plugin directly ---");
    manager.with_plugin_mut("hello", |_hello_plugin| {
        println!("  HelloPlugin can access other plugins via context.get_plugin()");
    })?;

    println!("\n--- Greeter plugin calls Hello plugin directly ---");
    manager.with_plugin_mut("greeter", |_greeter_plugin| {
        println!("  GreeterPlugin can access other plugins via context.get_plugin()");
    })?;

    println!("\n=== Phase 3: Interface Discovery ===");

    println!("\n--- Find all plugins implementing Greet interface ---");
    let greet_providers = manager.plugins_with_interface("Greet");
    println!("  Plugins implementing Greet: {:?}", greet_providers);

    println!("\n--- Find all plugins implementing Calculator interface ---");
    let calc_providers = manager.plugins_with_interface("Calculator");
    println!("  Plugins implementing Calculator: {:?}", calc_providers);

    println!("\n--- Find all plugins implementing Logger interface ---");
    let logger_providers = manager.plugins_with_interface("Logger");
    println!("  Plugins implementing Logger: {:?}", logger_providers);

    println!("\n=== Phase 4: Direct Method Calls ===");

    println!("\n--- Direct method calls on HelloPlugin ---");
    manager.with_plugin_mut("hello", |plugin| {
        let hello = plugin
            .downcast_mut::<plugin_types::HelloPlugin>()
            .expect("must be HelloPlugin");

        let greeting = hello.get_greeting().to_string();
        println!("  hello.get_greeting() = {}", greeting);

        hello.add_data("user".to_string(), "Alice".to_string());
        let user = hello.get_data_value("user").map(|s| s.as_str());
        println!("  hello.get_data_value(\"user\") = {:?}", user);
    })?;

    println!("\n--- Direct method calls on GreeterPlugin ---");
    manager.with_plugin("greeter", |plugin| {
        let greeter = plugin
            .downcast_ref::<plugin_types::GreeterPlugin>()
            .expect("must be GreeterPlugin");

        let count = greeter.get_count();
        println!("  greeter.get_count() = {}", count);

        let last = greeter.get_last_greeting();
        println!("  greeter.get_last_greeting() = {}", last);
    })?;

    println!("\n=== Phase 5: PluginInfo and PluginResult Objects ===");

    println!("\n--- Get PluginInfo for hello ---");
    let info = manager.get_plugin_info("hello")?;
    println!("  name: {}", info.name);
    println!("  version: {}", info.version);
    println!("  authors: {:?}", info.authors);
    println!("  interfaces: {:?}", info.interfaces);

    println!("\n--- Get PluginInfo for greeter ---");
    let info = manager.get_plugin_info("greeter")?;
    println!("  name: {}", info.name);
    println!("  version: {}", info.version);
    println!("  interfaces: {:?}", info.interfaces);

    println!("\n--- Get all plugin info ---");
    let all_info = manager.get_all_plugin_info();
    for info in &all_info {
        println!(
            "  {} v{} (interfaces: {:?})",
            info.name, info.version, info.interfaces
        );
    }

    println!("\n--- Check interface support ---");
    println!(
        "  hello has Greet: {}",
        manager.has_interface("hello", "Greet")
    );
    println!(
        "  hello has Calculator: {}",
        manager.has_interface("hello", "Calculator")
    );
    println!(
        "  greeter has Calculator: {}",
        manager.has_interface("greeter", "Calculator")
    );
    println!(
        "  greeter has Logger: {}",
        manager.has_interface("greeter", "Logger")
    );

    println!("\n--- Get plugin interfaces ---");
    let hello_ifaces = manager.get_plugin_interfaces("hello")?;
    println!("  hello interfaces: {:?}", hello_ifaces);
    let greeter_ifaces = manager.get_plugin_interfaces("greeter")?;
    println!("  greeter interfaces: {:?}", greeter_ifaces);

    println!("\n--- call_plugin_result returns PluginResult ---");
    // PluginResult is now built via with_plugin + direct method calls
    let result = manager.with_plugin("hello", |plugin| {
        let greet: &dyn Greet = plugin
            .downcast_ref::<plugin_types::HelloPlugin>()
            .expect("hello must implement Greet");
        plugin_system::PluginResult::String(greet.greet("World"))
    })?;
    println!("  result type: {}", std::any::type_name_of_val(&result));
    println!("  result value: {}", result);
    println!("  as_string: {:?}", result.as_string());

    println!("\n--- PluginResult type conversions ---");
    let s: plugin_system::PluginResult = "hello".into();
    println!("  from &str: {}", s);
    let i: plugin_system::PluginResult = 42i64.into();
    println!("  from i64: {}", i);
    let f: plugin_system::PluginResult = std::f64::consts::PI.into();
    println!("  from f64: {}", f);
    let b: plugin_system::PluginResult = true.into();
    println!("  from bool: {}", b);
    let l: plugin_system::PluginResult = vec!["a".to_string(), "b".to_string()].into();
    println!("  from Vec<String>: {}", l);
    let n: plugin_system::PluginResult = Option::<String>::None.into();
    println!("  from None: {}", n);

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
