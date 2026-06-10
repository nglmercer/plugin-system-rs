use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::Json,
    routing::{get, post},
    Router,
};
use sd_types::*;
use sd_events::{EventBus, StreamEvent};
use sd_actions::ActionRegistry;
use sd_profiles::ProfileManager;
use sd_devices::DeviceManager;
use sd_plugins::SdPluginManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
pub struct AppState {
    pub events: Arc<EventBus>,
    pub action_registry: Arc<RwLock<ActionRegistry>>,
    pub profile_manager: Arc<ProfileManager>,
    pub device_manager: Arc<DeviceManager>,
    pub plugin_manager: Arc<SdPluginManager>,
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
    let device = state.device_manager.get_device(&DeviceId(device_id.clone())).await;
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
    let actions: Vec<String> = registry.list().iter()
        .map(|a| format!("{} ({})", a.action_name(), a.category()))
        .collect();
    Json(ApiResponse::success(actions))
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
        let _ = tx.blocking_send(event.clone());
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
                                if let Some(dev) = state.device_manager.get_device(&DeviceId(device.to_string())).await {
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

    Router::new()
        .route("/api/devices", get(list_devices))
        .route("/api/devices/:device_id/press/:button_index", post(simulate_button_press))
        .route("/api/profiles", get(list_profiles).post(create_profile))
        .route("/api/profiles/:profile_id", get(get_profile).delete(delete_profile))
        .route("/api/actions", get(list_actions))
        .route("/api/plugins", get(list_plugins))
        .route("/api/plugins/reload", post(reload_plugins))
        .route("/ws", get(websocket_handler))
        .layer(cors)
        .with_state(state)
}
