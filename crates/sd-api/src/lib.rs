use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use plugin_key_simulator::KeySimulator;
use sd_actions::{ActionContext, ActionRegistry};
use sd_devices::DeviceManager;
use sd_events::{EventBus, StreamEvent};
use sd_plugins::SdPluginManager;
use sd_profiles::ProfileManager;
use sd_types::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

const DASHBOARD_CONFIG_PATH: &str = "data/dashboard.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DashboardWidget {
    pub id: String,
    #[serde(rename = "type")]
    pub widget_type: String,
    pub title: String,
    #[serde(rename = "colSpan")]
    pub col_span: u32,
    #[serde(rename = "rowSpan")]
    pub row_span: u32,
    #[serde(default)]
    pub settings: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DashboardLayout {
    pub widgets: Vec<DashboardWidget>,
    pub columns: u32,
}

impl Default for DashboardLayout {
    fn default() -> Self {
        Self {
            widgets: Vec::new(),
            columns: 3,
        }
    }
}

pub fn load_dashboard_config() -> DashboardLayout {
    std::fs::read_to_string(DASHBOARD_CONFIG_PATH)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn save_dashboard_config(layout: &DashboardLayout) -> bool {
    if let Some(parent) = std::path::Path::new(DASHBOARD_CONFIG_PATH).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(layout) {
        Ok(json) => std::fs::write(DASHBOARD_CONFIG_PATH, json).is_ok(),
        Err(_) => false,
    }
}

#[derive(Serialize)]
struct SystemStats {
    cpu_usage: f64,
    cpu_model: String,
    cpu_cores: usize,
    memory_total: u64,
    memory_used: u64,
    memory_usage: f64,
    swap_total: u64,
    swap_used: u64,
    load_avg: [f64; 3],
    uptime: u64,
    process_count: usize,
    thread_count: usize,
}

#[derive(Serialize)]
struct PluginDataResponse {
    name: String,
    version: String,
    interfaces: Vec<String>,
    data: serde_json::Value,
}

struct CpuTimes {
    idle: u64,
    total: u64,
}

fn read_cpu_times() -> Option<CpuTimes> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    let line = content.lines().next()?;
    let parts: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() >= 4 {
        let idle = parts[3];
        let total: u64 = parts.iter().sum();
        Some(CpuTimes { idle, total })
    } else {
        None
    }
}

fn read_cpu_usage_sample() -> f64 {
    let first = read_cpu_times();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let second = read_cpu_times();
    match (first, second) {
        (Some(a), Some(b)) => {
            let total_delta = b.total.saturating_sub(a.total);
            let idle_delta = b.idle.saturating_sub(a.idle);
            if total_delta > 0 {
                ((total_delta - idle_delta) as f64 / total_delta as f64 * 100.0).min(100.0)
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

fn read_cpu_model() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "Unknown CPU".to_string())
}

fn read_cpu_cores() -> usize {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .map(|content| {
            content
                .lines()
                .filter(|l| l.starts_with("processor"))
                .count()
        })
        .unwrap_or(1)
}

fn read_memory_info() -> (u64, u64, u64, u64) {
    let content = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut mem_total = 0u64;
    let mut mem_available = 0u64;
    let mut swap_total = 0u64;
    let mut swap_free = 0u64;

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let value = parts[1].parse::<u64>().unwrap_or(0) * 1024;
            match parts[0] {
                "MemTotal:" => mem_total = value,
                "MemAvailable:" => mem_available = value,
                "SwapTotal:" => swap_total = value,
                "SwapFree:" => swap_free = value,
                _ => {}
            }
        }
    }

    let mem_used = mem_total.saturating_sub(mem_available.min(mem_total));
    let swap_used = swap_total.saturating_sub(swap_free.min(swap_total));
    (mem_total, mem_used, swap_total, swap_used)
}

fn read_load_avg() -> [f64; 3] {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|content| {
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 3 {
                Some([
                    parts[0].parse().unwrap_or(0.0),
                    parts[1].parse().unwrap_or(0.0),
                    parts[2].parse().unwrap_or(0.0),
                ])
            } else {
                None
            }
        })
        .unwrap_or([0.0, 0.0, 0.0])
}

fn read_uptime() -> u64 {
    std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|content| {
            content
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<f64>().ok())
                .map(|v| v as u64)
        })
        .unwrap_or(0)
}

fn read_process_count() -> (usize, usize) {
    let mut processes = 0usize;
    let mut threads = 0usize;
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if let Some(s) = name.to_str() {
                if s.chars().all(|c| c.is_ascii_digit()) {
                    processes += 1;
                    // Count threads in /proc/<pid>/task
                    if let Ok(task_dir) = std::fs::read_dir(entry.path().join("task")) {
                        threads += task_dir.flatten().count();
                    }
                }
            }
        }
    }
    (processes, threads)
}

#[derive(Clone)]
pub struct AppState {
    pub events: Arc<EventBus>,
    pub action_registry: Arc<RwLock<ActionRegistry>>,
    pub profile_manager: Arc<ProfileManager>,
    pub device_manager: Arc<DeviceManager>,
    pub plugin_manager: Arc<SdPluginManager>,
    pub dashboard_config: Arc<RwLock<DashboardLayout>>,
}

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

// Device endpoints
async fn list_devices(State(state): State<AppState>) -> Json<ApiResponse<Vec<DeviceInfo>>> {
    let devices = state.device_manager.list_devices().await;
    Json(ApiResponse::success(devices))
}

async fn simulate_button_press(
    State(state): State<AppState>,
    Path((device_id, button_index)): Path<(String, usize)>,
) -> Json<ApiResponse<String>> {
    let device = state
        .device_manager
        .get_device(&DeviceId(device_id.clone()))
        .await;
    if let Some(device) = device {
        device.press_button(button_index);
        state.events.emit(StreamEvent::ButtonPressed {
            device: DeviceId(device_id),
            index: button_index,
            profile: ProfileId(uuid::Uuid::nil()),
        });
        Json(ApiResponse::success("Button pressed".to_string()))
    } else {
        Json(ApiResponse::error("Device not found"))
    }
}

// Profile endpoints
async fn list_profiles(State(state): State<AppState>) -> Json<ApiResponse<Vec<Profile>>> {
    let profiles = state.profile_manager.list_profiles().await;
    Json(ApiResponse::success(profiles))
}

#[derive(Deserialize)]
struct CreateProfileRequest {
    name: String,
}

async fn create_profile(
    State(state): State<AppState>,
    Json(req): Json<CreateProfileRequest>,
) -> Json<ApiResponse<ProfileId>> {
    let id = state.profile_manager.create_profile(req.name).await;
    Json(ApiResponse::success(id))
}

async fn get_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Json<ApiResponse<Profile>> {
    let id = ProfileId(uuid::Uuid::parse_str(&profile_id).unwrap_or(uuid::Uuid::nil()));
    match state.profile_manager.get_profile(&id).await {
        Some(profile) => Json(ApiResponse::success(profile)),
        None => Json(ApiResponse::error("Profile not found")),
    }
}

async fn delete_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Json<ApiResponse<bool>> {
    let id = ProfileId(uuid::Uuid::parse_str(&profile_id).unwrap_or(uuid::Uuid::nil()));
    let deleted = state.profile_manager.delete_profile(&id).await;
    Json(ApiResponse::success(deleted))
}

// Action endpoints
async fn list_actions(State(state): State<AppState>) -> Json<ApiResponse<Vec<String>>> {
    let registry = state.action_registry.read().await;
    let actions: Vec<String> = registry
        .list()
        .iter()
        .map(|a| format!("{} ({})", a.action_name(), a.category()))
        .collect();
    Json(ApiResponse::success(actions))
}

#[derive(Deserialize)]
struct ExecuteActionRequest {
    action_name: String,
}

async fn execute_action(
    State(state): State<AppState>,
    Json(req): Json<ExecuteActionRequest>,
) -> Json<ApiResponse<String>> {
    let registry = state.action_registry.read().await;
    match registry.find_by_name(&req.action_name) {
        Some(action) => {
            let ctx = ActionContext {
                device_id: DeviceId("web".to_string()),
                button_index: 0,
                profile_id: ProfileId(uuid::Uuid::nil()),
                state: Arc::new(RwLock::new(std::collections::HashMap::new())),
                events: state.events.clone(),
            };
            let result = action.execute(&ctx);
            Json(ApiResponse::success(result.to_string()))
        }
        None => Json(ApiResponse::error("Action not found")),
    }
}

// Hotkey send endpoint
#[derive(Deserialize)]
struct SendHotkeyRequest {
    keys: Vec<String>,
}

#[derive(Serialize)]
struct SendHotkeyResponse {
    combo: String,
    keys: Vec<String>,
    simulated: bool,
    error: Option<String>,
}

async fn send_hotkey(
    State(state): State<AppState>,
    Json(req): Json<SendHotkeyRequest>,
) -> Json<ApiResponse<SendHotkeyResponse>> {
    let combo = req.keys.join("+");
    let keys_for_sim = req.keys.clone();

    let sim_result = tokio::task::spawn_blocking(move || {
        let sim = plugin_key_simulator::KeySimulatorPlugin::new();
        sim.simulate_keys(&keys_for_sim)
    })
    .await
    .unwrap_or(Err("Blocking task failed".to_string()));

    let (simulated, sim_error) = match sim_result {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e)),
    };

    state.events.emit(StreamEvent::ActionExecuted {
        action: sd_types::ActionId("hotkey".to_string()),
        result: sd_types::PluginResult::string(format!("Keys: {}", combo)),
    });

    if let Some(ref err) = sim_error {
        log::warn!("[Hotkey] simulation failed for {}: {}", combo, err);
    } else {
        log::info!("[Hotkey] simulation succeeded for {}", combo);
    }

    Json(ApiResponse::success(SendHotkeyResponse {
        combo,
        keys: req.keys,
        simulated,
        error: sim_error,
    }))
}

// Hotkey record endpoint
#[derive(Deserialize)]
struct RecordHotkeyRequest {
    #[serde(default)]
    _device: Option<String>,
    #[serde(default = "default_timeout")]
    timeout_ms: u64,
}

fn default_timeout() -> u64 { 10000 }

#[derive(Serialize)]
struct RecordHotkeyResponse {
    combo: String,
}

async fn record_hotkey(
    Json(req): Json<RecordHotkeyRequest>,
) -> Json<ApiResponse<RecordHotkeyResponse>> {
    let result = tokio::task::spawn_blocking(move || {
        plugin_key_simulator::KeySimulatorPlugin::listen_for_combo(req.timeout_ms)
    })
    .await
    .unwrap_or(Err("Blocking task failed".to_string()));

    match result {
        Ok(combo) => {
            log::info!("[Hotkey] Recorded combo: {}", combo);
            Json(ApiResponse::success(RecordHotkeyResponse { combo }))
        }
        Err(e) => Json(ApiResponse::error(e)),
    }
}

// Hotkey input devices endpoint
#[derive(Serialize)]
struct InputDeviceInfo {
    path: String,
    name: String,
}

async fn list_input_devices() -> Json<ApiResponse<Vec<InputDeviceInfo>>> {
    let devices = plugin_key_simulator::KeySimulatorPlugin::list_input_devices();
    let info: Vec<InputDeviceInfo> = devices.into_iter()
        .map(|(path, name)| InputDeviceInfo { path, name })
        .collect();
    Json(ApiResponse::success(info))
}

// Plugin endpoints
async fn list_plugins(State(state): State<AppState>) -> Json<ApiResponse<Vec<String>>> {
    let plugins = state.plugin_manager.list_plugins().await;
    Json(ApiResponse::success(plugins))
}

async fn reload_plugins(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    match state.plugin_manager.reload_plugins().await {
        Ok(()) => Json(ApiResponse::success("Plugins reloaded".to_string())),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

async fn get_plugin_data(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
) -> Json<ApiResponse<PluginDataResponse>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    match manager.get_plugin_info(&plugin_name) {
        Ok(info) => {
            let data = match plugin_name.as_str() {
                "system-monitor" => {
                    let cpu = read_cpu_usage_sample();
                    let (mem_total, mem_used, swap_total, swap_used) = read_memory_info();
                    let load = read_load_avg();
                    serde_json::json!({
                        "cpu_usage": cpu,
                        "memory_total": mem_total,
                        "memory_used": mem_used,
                        "memory_usage": if mem_total > 0 { mem_used as f64 / mem_total as f64 * 100.0 } else { 0.0 },
                        "swap_total": swap_total,
                        "swap_used": swap_used,
                        "load_avg": load,
                    })
                }
                _ => serde_json::json!({}),
            };

            Json(ApiResponse::success(PluginDataResponse {
                name: info.name,
                version: info.version,
                interfaces: info.interfaces,
                data,
            }))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

// System stats endpoint
async fn get_system_stats() -> Json<ApiResponse<SystemStats>> {
    let cpu_usage = read_cpu_usage_sample();
    let cpu_model = read_cpu_model();
    let cpu_cores = read_cpu_cores();
    let (mem_total, mem_used, swap_total, swap_used) = read_memory_info();
    let load_avg = read_load_avg();
    let uptime = read_uptime();
    let (process_count, thread_count) = read_process_count();

    let memory_usage = if mem_total > 0 {
        mem_used as f64 / mem_total as f64 * 100.0
    } else {
        0.0
    };

    Json(ApiResponse::success(SystemStats {
        cpu_usage,
        cpu_model,
        cpu_cores,
        memory_total: mem_total,
        memory_used: mem_used,
        memory_usage,
        swap_total,
        swap_used,
        load_avg,
        uptime,
        process_count,
        thread_count,
    }))
}

// Dashboard config endpoints
async fn get_dashboard(State(state): State<AppState>) -> Json<ApiResponse<DashboardLayout>> {
    let layout = state.dashboard_config.read().await;
    Json(ApiResponse::success(layout.clone()))
}

async fn save_dashboard(
    State(state): State<AppState>,
    Json(layout): Json<DashboardLayout>,
) -> Json<ApiResponse<bool>> {
    {
        let mut config = state.dashboard_config.write().await;
        *config = layout.clone();
    }
    let ok = save_dashboard_config(&layout);
    if ok {
        Json(ApiResponse::success(true))
    } else {
        Json(ApiResponse::error("Failed to save dashboard config"))
    }
}

// WebSocket handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: axum::extract::ws::WebSocket, state: AppState) {
    use axum::extract::ws::Message;
    use futures_util::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();

    // Send initial state
    let devices = state.device_manager.list_devices().await;
    let initial_state = serde_json::json!({
        "type": "initial_state",
        "devices": devices,
    });
    let _ = sender.send(Message::Text(initial_state.to_string())).await;

    // Subscribe to events
    let (tx, mut rx) = tokio::sync::mpsc::channel::<StreamEvent>(100);
    let events = state.events.clone();

    events.subscribe_all(move |event| {
        let _ = tx.try_send(event.clone());
    });

    // Forward events to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let msg = serde_json::json!({
                "type": "event",
                "event": event,
            });
            if sender.send(Message::Text(msg.to_string())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(msg_type) = parsed.get("type").and_then(|t| t.as_str()) {
                        if msg_type == "press_button" {
                            if let (Some(device), Some(index)) = (
                                parsed.get("device").and_then(|d| d.as_str()),
                                parsed.get("index").and_then(|i| i.as_u64()),
                            ) {
                                if let Some(dev) = state
                                    .device_manager
                                    .get_device(&DeviceId(device.to_string()))
                                    .await
                                {
                                    dev.press_button(index as usize);
                                }
                            }
                        }
                    }
                }
            }
            Ok(Message::Close(_frame)) => {
                break;
            }
            _ => {}
        }
    }

    send_task.abort();
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let static_files =
        ServeDir::new("web/dist").not_found_service(get(|_: axum::extract::Request| async {
            Html(include_str!("../../../web/dist/index.html"))
        }));

    Router::new()
        .route("/api/devices", get(list_devices))
        .route(
            "/api/devices/:device_id/press/:button_index",
            post(simulate_button_press),
        )
        .route("/api/profiles", get(list_profiles).post(create_profile))
        .route(
            "/api/profiles/:profile_id",
            get(get_profile).delete(delete_profile),
        )
        .route("/api/actions", get(list_actions).post(execute_action))
        .route("/api/hotkey/send", post(send_hotkey))
        .route("/api/hotkey/record", post(record_hotkey))
        .route("/api/hotkey/devices", get(list_input_devices))
        .route("/api/plugins", get(list_plugins))
        .route("/api/plugins/reload", post(reload_plugins))
        .route("/api/plugins/:plugin_name", get(get_plugin_data))
        .route("/api/system-stats", get(get_system_stats))
        .route("/api/dashboard", get(get_dashboard).put(save_dashboard))
        .route("/ws", get(websocket_handler))
        .nest_service("/", static_files)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
