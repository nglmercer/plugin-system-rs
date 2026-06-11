use axum::{
    response::Html,
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::{
    api::{actions, dashboard_handlers, devices, hotkeys, plugins, profiles, system, websocket},
    state::AppState,
};

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let static_files =
        ServeDir::new("web/dist").not_found_service(get(|_: axum::extract::Request| async {
            Html(include_str!("../../../../web/dist/index.html"))
        }));

    Router::new()
        .route("/api/devices", get(devices::list_devices))
        .route(
            "/api/devices/:device_id/press/:button_index",
            post(devices::simulate_button_press),
        )
        .route(
            "/api/profiles",
            get(profiles::list_profiles).post(profiles::create_profile),
        )
        .route(
            "/api/profiles/:profile_id",
            get(profiles::get_profile).delete(profiles::delete_profile),
        )
        .route(
            "/api/actions",
            get(actions::list_actions).post(actions::execute_action),
        )
        .route("/api/hotkey/send", post(hotkeys::send_hotkey))
        .route("/api/hotkey/record", post(hotkeys::record_hotkey))
        .route(
            "/api/hotkey/record/reset",
            post(hotkeys::reset_hotkey_recording),
        )
        .route("/api/plugins", get(plugins::list_plugins))
        .route("/api/plugins/reload", post(plugins::reload_plugins))
        .route("/api/plugins/:plugin_name", get(plugins::get_plugin_data))
        .route("/api/system-stats", get(system::get_system_stats))
        .route(
            "/api/dashboard",
            get(dashboard_handlers::get_dashboard).put(dashboard_handlers::save_dashboard),
        )
        .route("/ws", get(websocket::websocket_handler))
        .nest_service("/", static_files)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
