use axum::{
    extract::{Path, State},
    Json,
};
use sd_types::ProfileId;

use crate::{response::ApiResponse, state::AppState};

pub(crate) async fn list_profiles(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<sd_types::Profile>>> {
    let profiles = state.profile_manager.list_profiles().await;
    Json(ApiResponse::success(profiles))
}

#[derive(serde::Deserialize)]
pub(crate) struct CreateProfileRequest {
    name: String,
}

pub(crate) async fn create_profile(
    State(state): State<AppState>,
    Json(req): Json<CreateProfileRequest>,
) -> Json<ApiResponse<ProfileId>> {
    let id = state.profile_manager.create_profile(req.name).await;
    Json(ApiResponse::success(id))
}

pub(crate) async fn get_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Json<ApiResponse<sd_types::Profile>> {
    let id = ProfileId(uuid::Uuid::parse_str(&profile_id).unwrap_or(uuid::Uuid::nil()));

    match state.profile_manager.get_profile(&id).await {
        Some(profile) => Json(ApiResponse::success(profile)),
        None => Json(ApiResponse::error("Profile not found")),
    }
}

pub(crate) async fn delete_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Json<ApiResponse<bool>> {
    let id = ProfileId(uuid::Uuid::parse_str(&profile_id).unwrap_or(uuid::Uuid::nil()));
    let deleted = state.profile_manager.delete_profile(&id).await;
    Json(ApiResponse::success(deleted))
}
