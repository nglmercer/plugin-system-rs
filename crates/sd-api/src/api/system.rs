use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{response::ApiResponse, state::AppState};

#[derive(Serialize, Deserialize)]
pub(crate) struct SystemStats {
    pub cpu_usage: f64,
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_usage: f64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub load_avg: [f64; 3],
    pub uptime: u64,
    pub process_count: usize,
    pub thread_count: usize,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LocalIpInfo {
    pub ip: String,
    pub port: u16,
    pub url: String,
}

pub(crate) async fn get_system_stats(
    State(state): State<AppState>,
) -> Json<ApiResponse<SystemStats>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    let stats = if let Ok(plugin_arc) = manager.get_plugin_arc("system-monitor") {
        let _ = crate::api::helpers::call_plugin_ok(
            &manager,
            "system-monitor",
            "refresh",
            serde_json::json!({}),
        )
        .await;
        let plugin = plugin_arc.read().expect("plugin lock poisoned");
        if let Some(data) = plugin.interface_data() {
            serde_json::from_value(data).unwrap_or_else(|_| SystemStats::default())
        } else {
            SystemStats::default()
        }
    } else {
        SystemStats::default()
    };

    Json(ApiResponse::success(stats))
}

pub(crate) async fn get_local_ip() -> Json<ApiResponse<LocalIpInfo>> {
    let ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = 3000u16;
    let url = format!("http://{}:{}", ip, port);
    Json(ApiResponse::success(LocalIpInfo { ip, port, url }))
}

impl Default for SystemStats {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            cpu_model: "Unknown".to_string(),
            cpu_cores: 0,
            memory_total: 0,
            memory_used: 0,
            memory_usage: 0.0,
            swap_total: 0,
            swap_used: 0,
            load_avg: [0.0, 0.0, 0.0],
            uptime: 0,
            process_count: 0,
            thread_count: 0,
        }
    }
}
