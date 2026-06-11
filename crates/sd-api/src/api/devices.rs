use axum::{
    extract::{Path, State},
    Json,
};
use sd_events::StreamEvent;
use sd_types::{DeviceId, ProfileId};

use crate::{response::ApiResponse, state::AppState};

pub(crate) async fn list_devices(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<sd_types::DeviceInfo>>> {
    let devices = state.device_manager.list_devices().await;
    Json(ApiResponse::success(devices))
}

pub(crate) async fn simulate_button_press(
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
