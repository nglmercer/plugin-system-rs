use axum::{extract::State, Json};

use crate::{response::ApiResponse, state::AppState};

pub(crate) async fn get_dashboard(
    State(state): State<AppState>,
) -> Json<ApiResponse<crate::api::DashboardLayout>> {
    let layout = state.dashboard_config.read().await;
    Json(ApiResponse::success(layout.clone()))
}

pub(crate) async fn save_dashboard(
    State(state): State<AppState>,
    Json(layout): Json<crate::api::DashboardLayout>,
) -> Json<ApiResponse<bool>> {
    {
        let mut config = state.dashboard_config.write().await;
        *config = layout.clone();
    }

    let ok = crate::api::dashboard::save_dashboard_config(&layout);
    if ok {
        Json(ApiResponse::success(true))
    } else {
        Json(ApiResponse::error("Failed to save dashboard config"))
    }
}
