use sd_events::EventBus;
use sd_actions::{ActionRegistry, HotkeyAction, TextAction, OpenUrlAction};
use sd_profiles::ProfileManager;
use sd_devices::{DeviceManager, VirtualDevice};
use sd_plugins::SdPluginManager;
use sd_api::{AppState, create_router, load_dashboard_config};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    println!("=== StreamDeck Core ===");
    println!("Starting up...\n");

    // Initialize event bus
    let events = Arc::new(EventBus::new());

    // Initialize action registry with built-in actions
    let action_registry = Arc::new(RwLock::new(ActionRegistry::new()));
    {
        let mut registry = action_registry.write().await;
        registry.register(Arc::new(HotkeyAction::new("ctrl+c")));
        registry.register(Arc::new(TextAction::new("Hello, World!")));
        registry.register(Arc::new(OpenUrlAction::new("https://example.com")));
    }

    // Initialize profile manager
    let profile_manager = Arc::new(ProfileManager::new(events.clone()));

    // Create a default profile
    let default_profile_id = profile_manager.create_profile("Default").await;
    println!("Created default profile: {:?}", default_profile_id);

    // Initialize device manager
    let device_manager = Arc::new(DeviceManager::new(events.clone()));

    // Add a virtual device (15 buttons like StreamDeck MK.2)
    let virtual_device = Arc::new(VirtualDevice::new("virtual-1", 15, events.clone()));
    device_manager.add_device(virtual_device).await;
    println!("Added virtual device with 15 buttons");

    // Initialize plugin manager
    let plugin_manager = Arc::new(SdPluginManager::new(
        events.clone(),
        action_registry.clone(),
    ));

    // Load plugins from directory
    let plugin_dir = "./plugins";
    match plugin_manager.load_plugins_from_dir(plugin_dir).await {
        Ok(loaded) => {
            if loaded.is_empty() {
                println!("No plugins found in {}", plugin_dir);
            } else {
                println!("Loaded {} plugins: {:?}", loaded.len(), loaded);
            }
        }
        Err(e) => {
            println!("Warning: Failed to load plugins: {}", e);
        }
    }

    // Create API state
    let dashboard_config = Arc::new(RwLock::new(load_dashboard_config()));
    let state = AppState {
        events: events.clone(),
        action_registry,
        profile_manager,
        device_manager,
        plugin_manager,
        dashboard_config,
    };

    // Start event bus listener
    let events_clone = events.clone();
    tokio::spawn(async move {
        events_clone.run().await;
    });

    // Start HTTP server
    let addr = "0.0.0.0:3000";
    println!("\nStarting HTTP server on http://{}", addr);
    println!("WebSocket endpoint: ws://{}/ws", addr);
    println!("API docs: http://{}/api", addr);
    println!("\nPress Ctrl+C to stop\n");

    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
