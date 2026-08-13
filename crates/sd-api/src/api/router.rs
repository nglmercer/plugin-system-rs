use axum::{
    response::{Html, IntoResponse},
    routing::{get, post, put},
    Router,
};
use sd_paths::resolve_web_dist;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::{
    api::{
        actions, dashboard_handlers, devices, hotkeys, icons, obs, plugins, profiles, proxy, system,
        volume, websocket,
    },
    auth,
    state::AppState,
};

/// Origins allowed to call the API cross-site during frontend development.
///
/// The shipped dashboard is served from this same origin and needs no CORS at
/// all. The old `allow_origin(Any)` existed only so `vite dev` on :5173 could
/// reach the daemon, and it bought that convenience by letting *every* page on
/// the internet script the API from the user's browser. Naming the dev servers
/// keeps the convenience without the hole. Set `SD_CORS_ALLOWED_ORIGINS` (comma
/// separated) to add your own.
fn allowed_origins() -> Vec<axum::http::HeaderValue> {
    let configured = std::env::var("SD_CORS_ALLOWED_ORIGINS").unwrap_or_default();
    let mut origins: Vec<String> = configured
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    origins.extend(
        [
            "http://localhost:5173",
            "http://127.0.0.1:5173",
        ]
        .into_iter()
        .map(String::from),
    );

    origins
        .into_iter()
        .filter_map(|origin| origin.parse().ok())
        .collect()
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(allowed_origins())
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    // Honour an explicit override, otherwise probe a sequence of likely
    // locations (exe-relative first, then CWD-relative). This makes the
    // router work whether sd-core was launched from the install dir, the
    // Start Menu, or a dev checkout.
    let web_dist = std::env::var("SD_WEB_DIST")
        .ok()
        .filter(|s| !s.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(resolve_web_dist);
    let web_dist_path = web_dist.clone();

    let static_files =
        ServeDir::new(&web_dist).not_found_service(get(move |_: axum::extract::Request| {
            let web_dist_path = web_dist_path.clone();
            async move {
                let index = web_dist_path.join("index.html");
                match std::fs::read_to_string(&index) {
                    Ok(body) => Html(body).into_response(),
                    Err(_) => Html("<h1>Not Found</h1>".to_string()).into_response(),
                }
            }
        }));

    // Everything that can act on the host, read its state, or reach the
    // network on the caller's behalf. All of it sits behind the token check.
    let protected = Router::new()
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
        .route("/api/actions/open-url", post(actions::open_url))
        .route("/api/hotkey/send", post(hotkeys::send_hotkey))
        .route("/api/hotkey/record", post(hotkeys::record_hotkey))
        .route(
            "/api/hotkey/record/reset",
            post(hotkeys::reset_hotkey_recording),
        )
        .route("/api/plugins", get(plugins::list_plugins))
        .route("/api/plugins/upload", post(plugins::upload_plugin))
        .route("/api/plugins/refresh", post(plugins::refresh_plugins))
        .route("/api/plugins/reload", post(plugins::reload_plugins))
        .route(
            "/api/plugins/:plugin_name/update",
            post(plugins::update_plugin),
        )
        .route(
            "/api/plugins/:plugin_name",
            get(plugins::get_plugin_data).delete(plugins::uninstall_plugin),
        )
        .route(
            "/api/plugins/:plugin_name/enabled",
            put(plugins::set_plugin_enabled),
        )
        .route(
            "/api/plugins/:plugin_name/command",
            post(plugins::call_plugin_command),
        )
        .route("/api/system-stats", get(system::get_system_stats))
        .route("/api/local-ip", get(system::get_local_ip))
        // Resolves a freedesktop icon name (as reported by an audio stream)
        // against the system icon theme. 404 is a normal answer.
        .route("/api/icon/:name", get(icons::get_icon))
        .route("/api/volume", get(volume::get_volume_state))
        .route("/api/volume/master", put(volume::set_master_volume))
        .route("/api/volume/mute", put(volume::set_master_mute))
        .route("/api/volume/apps", get(volume::get_app_volumes))
        .route("/api/volume/app/volume", put(volume::set_app_volume))
        .route("/api/volume/app/mute", put(volume::set_app_mute))
        .route("/api/obs/status", get(obs::get_obs_status))
        .route("/api/obs/connect", post(obs::connect_obs))
        .route("/api/obs/disconnect", post(obs::disconnect_obs))
        .route("/api/obs/stream/start", post(obs::start_stream))
        .route("/api/obs/stream/stop", post(obs::stop_stream))
        .route("/api/obs/record/start", post(obs::start_record))
        .route("/api/obs/record/stop", post(obs::stop_record))
        .route("/api/obs/record/pause", post(obs::toggle_record_pause))
        .route("/api/obs/scenes", get(obs::get_scenes))
        .route("/api/obs/scenes/current", post(obs::set_current_scene))
        .route("/api/obs/inputs", get(obs::get_inputs))
        .route("/api/obs/inputs/volume", put(obs::set_input_volume))
        .route("/api/obs/inputs/mute", put(obs::set_input_mute))
        .route("/api/obs/virtualcam/toggle", post(obs::toggle_virtual_cam))
        .route("/api/obs/replay/start", post(obs::start_replay_buffer))
        .route("/api/obs/replay/stop", post(obs::stop_replay_buffer))
        .route("/api/obs/replay/save", post(obs::save_replay))
        .route("/api/obs/transitions", get(obs::get_transitions))
        .route("/api/obs/transitions/current", post(obs::set_transition))
        .route("/api/obs/scene-items", get(obs::get_scene_items))
        .route(
            "/api/obs/scene-item/enabled",
            put(obs::set_scene_item_enabled),
        )
        .route(
            "/api/obs/studio-mode",
            get(obs::get_studio_mode).post(obs::set_studio_mode),
        )
        .route("/api/obs/preview/current", post(obs::set_preview_scene))
        .route(
            "/api/obs/studio-mode/transition",
            post(obs::trigger_studio_transition),
        )
        .route("/api/obs/media-inputs", get(obs::get_media_inputs))
        .route(
            "/api/obs/media-inputs/action",
            post(obs::media_input_action),
        )
        .route("/api/obs/filters", get(obs::get_filters))
        .route("/api/obs/filters/enabled", put(obs::set_filter_enabled))
        .route("/api/obs/profiles", get(obs::get_profiles))
        .route("/api/obs/profiles/current", post(obs::set_profile))
        .route("/api/obs/scene-collections", get(obs::get_scene_collections))
        .route(
            "/api/obs/scene-collections/current",
            post(obs::set_scene_collection),
        )
        .route("/api/obs/screenshot/save", post(obs::save_screenshot))
        .route(
            "/api/dashboard",
            get(dashboard_handlers::get_dashboard).put(dashboard_handlers::save_dashboard),
        )
        .route("/api/proxy", post(proxy::proxy_handler))
        .route("/ws", get(websocket::websocket_handler))
        // `route_layer` rather than `layer`: the check must apply to these
        // routes only, and must not run for requests that fall through to the
        // static file service below.
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::require_token,
        ));

    // The only unauthenticated endpoint, and it enforces its own rule: it
    // answers loopback callers only. The dashboard is served from this origin
    // and has no other way to bootstrap the token.
    let public = Router::new().route("/api/auth/token", get(auth::get_token));

    protected
        .merge(public)
        .nest_service("/", static_files)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
