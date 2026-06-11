use std::{collections::HashMap, sync::Arc};

use axum::{extract::State, Json};
use sd_actions::ActionContext;
use sd_types::{DeviceId, ProfileId};
use tokio::sync::RwLock;

use crate::{response::ApiResponse, state::AppState};

pub(crate) async fn list_actions(State(state): State<AppState>) -> Json<ApiResponse<Vec<String>>> {
    let registry = state.action_registry.read().await;
    let actions: Vec<String> = registry
        .list()
        .iter()
        .map(|a| format!("{} ({})", a.action_name(), a.category()))
        .collect();

    Json(ApiResponse::success(actions))
}

#[derive(serde::Deserialize)]
pub(crate) struct ExecuteActionRequest {
    action_name: String,
}

pub(crate) async fn execute_action(
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
                state: Arc::new(RwLock::new(HashMap::new())),
                events: state.events.clone(),
            };
            let result = action.execute(&ctx);
            Json(ApiResponse::success(result.to_string()))
        }
        None => Json(ApiResponse::error("Action not found")),
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct OpenUrlRequest {
    url: String,
}

pub(crate) async fn open_url(Json(req): Json<OpenUrlRequest>) -> Json<ApiResponse<String>> {
    if req.url.is_empty() {
        return Json(ApiResponse::error("No URL provided"));
    }

    log::info!("[OpenUrl] Opening: {}", req.url);

    if let Err(e) = open::that(&req.url) {
        log::error!("[OpenUrl] Failed: {}", e);
        return Json(ApiResponse::error(&format!("Failed to open: {}", e)));
    }

    Json(ApiResponse::success(format!("Opened: {}", req.url)))
}
