use sd_actions::{ActionRegistry, HotkeyAction, OpenUrlAction, TextAction};
use sd_api::{create_router, load_dashboard_config, AppState};
use sd_devices::{DeviceManager, VirtualDevice};
use sd_events::EventBus;
use sd_plugins::SdPluginManager;
use sd_profiles::ProfileManager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

mod tray;

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

    let shutdown = Arc::new(AtomicBool::new(false));

    let shutdown_clone = shutdown.clone();
    ctrlc_handler(shutdown_clone);

    let shutdown_tray = shutdown.clone();
    tray::spawn_tray(shutdown_tray);

    let events = Arc::new(EventBus::new());

    let action_registry = Arc::new(RwLock::new(ActionRegistry::new()));
    {
        let mut registry = action_registry.write().await;
        registry.register(Arc::new(HotkeyAction::new("ctrl+c")));
        registry.register(Arc::new(TextAction::new("Hello, World!")));
        registry.register(Arc::new(OpenUrlAction::new("https://example.com")));
    }

    let profile_manager = Arc::new(ProfileManager::new(events.clone()));

    let default_profile_id = profile_manager.create_profile("Default").await;
    println!("Created default profile: {:?}", default_profile_id);

    let device_manager = Arc::new(DeviceManager::new(events.clone()));

    let virtual_device = Arc::new(VirtualDevice::new("virtual-1", 15, events.clone()));
    device_manager.add_device(virtual_device).await;
    println!("Added virtual device with 15 buttons");

    let plugin_manager = Arc::new(SdPluginManager::new(
        events.clone(),
        action_registry.clone(),
    ));

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

    let dashboard_config = Arc::new(RwLock::new(load_dashboard_config()));
    let state = AppState {
        events: events.clone(),
        action_registry,
        profile_manager,
        device_manager,
        plugin_manager,
        dashboard_config,
    };

    let events_clone = events.clone();
    tokio::spawn(async move {
        events_clone.run().await;
    });

    let addr = "0.0.0.0:3000";
    let local_ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    println!("\nStarting HTTP server on http://{}", addr);
    println!("Local access: http://localhost:3000");
    println!("Network access: http://{}:3000", local_ip);
    println!("WebSocket endpoint: ws://{}/ws", addr);
    println!("API docs: http://{}/api", addr);
    println!("\nSystem tray icon active. Right-click for options.");
    println!("Press Ctrl+C to stop\n");

    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn ctrlc_handler(shutdown: Arc<AtomicBool>) {
    let _ = ctrlc::set_handler(move || {
        println!("\nShutting down...");
        shutdown.store(true, Ordering::Relaxed);
        std::process::exit(0);
    });
}
