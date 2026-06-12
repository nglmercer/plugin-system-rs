use sd_actions::{ActionRegistry, HotkeyAction, OpenUrlAction, TextAction};
use sd_api::{create_router, load_dashboard_config, AppState};
use sd_devices::{DeviceManager, VirtualDevice};
use sd_events::EventBus;
use sd_plugins::SdPluginManager;
use sd_profiles::ProfileManager;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
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

    let pid_lock = acquire_pid_lock()?;
    tracing::info!(
        pid = std::process::id(),
        path = %pid_lock.path().display(),
        "Acquired pid lock"
    );

    println!("sd-core starting up...");

    let shutdown = Arc::new(AtomicBool::new(false));

    let shutdown_clone = shutdown.clone();
    ctrlc_handler(shutdown_clone, pid_lock.path().to_path_buf());

    let shutdown_tray = shutdown.clone();
    tray::spawn_tray(shutdown_tray, pid_lock.path().to_path_buf());

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

fn ctrlc_handler(shutdown: Arc<AtomicBool>, pid_lock_path: PathBuf) {
    let _ = ctrlc::set_handler(move || {
        tracing::info!("Shutting down...");
        shutdown.store(true, Ordering::Relaxed);
        remove_pid_lock(&pid_lock_path);
        std::process::exit(0);
    });
}

struct PidLock {
    path: PathBuf,
    pid: u32,
}

impl PidLock {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PidLock {
    fn drop(&mut self) {
        if let Ok(contents) = fs::read_to_string(&self.path) {
            if contents.trim() == self.pid.to_string() {
                remove_pid_lock(&self.path);
            }
        }
    }
}

fn acquire_pid_lock() -> Result<PidLock, Box<dyn std::error::Error>> {
    let path = pid_lock_path();
    let pid = std::process::id();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if let Some(existing_pid) = read_pid_lock(&path) {
        if existing_pid == pid {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("sd-core already holds this pid lock: {}", path.display()),
            )
            .into());
        }

        if pid_is_running(existing_pid) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!(
                    "sd-core is already running with PID {}. Lock file: {}",
                    existing_pid,
                    path.display()
                ),
            )
            .into());
        }

        tracing::warn!(
            pid = existing_pid,
            path = %path.display(),
            "Removing stale pid lock"
        );
        let _ = fs::remove_file(&path);
    }

    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => {
            writeln!(file, "{}", pid)?;
            Ok(PidLock { path, pid })
        }
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            if let Some(existing_pid) = read_pid_lock(&path) {
                if pid_is_running(existing_pid) {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        format!(
                            "sd-core is already running with PID {}. Lock file: {}",
                            existing_pid,
                            path.display()
                        ),
                    )
                    .into());
                }

                let _ = fs::remove_file(&path);
                return acquire_pid_lock();
            }

            Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("sd-core is already running. Lock file: {}", path.display()),
            )
            .into())
        }
        Err(err) => Err(err.into()),
    }
}

fn read_pid_lock(path: &Path) -> Option<u32> {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| contents.trim().parse::<u32>().ok())
}

fn pid_lock_path() -> PathBuf {
    if let Ok(path) = std::env::var("SD_CORE_PID_LOCK") {
        return PathBuf::from(path);
    }

    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir)
            .join("sd-core")
            .join("sd-core.pid.lock");
    }

    let uid = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|contents| {
            contents
                .lines()
                .find(|line| line.starts_with("Uid:"))
                .and_then(|line| line.split_whitespace().nth(1).map(str::to_string))
        })
        .unwrap_or_else(|| "unknown".to_string());

    std::env::temp_dir()
        .join(format!("sd-core-{uid}"))
        .join("sd-core.pid.lock")
}

fn pid_is_running(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        Path::new("/proc").join(pid.to_string()).exists()
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        true
    }
}

fn remove_pid_lock(path: &Path) {
    if let Err(err) = fs::remove_file(path) {
        if err.kind() != io::ErrorKind::NotFound {
            tracing::warn!(path = %path.display(), error = %err, "Failed to remove pid lock");
        }
    }
}
