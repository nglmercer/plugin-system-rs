use axum::Json;
use serde::Serialize;

use crate::response::ApiResponse;

struct CpuTimes {
    idle: u64,
    total: u64,
}

#[derive(Serialize)]
pub(crate) struct SystemStats {
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

pub(crate) fn read_cpu_usage_sample() -> f64 {
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

pub(crate) fn read_cpu_model() -> String {
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

pub(crate) fn read_cpu_cores() -> usize {
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
                    if let Ok(task_dir) = std::fs::read_dir(entry.path().join("task")) {
                        threads += task_dir.flatten().count();
                    }
                }
            }
        }
    }

    (processes, threads)
}

pub(crate) fn system_stats_data() -> serde_json::Value {
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

pub(crate) async fn get_system_stats() -> Json<ApiResponse<SystemStats>> {
    let stats = collect_system_stats();

    Json(ApiResponse::success(stats))
}

pub(crate) fn collect_system_stats() -> SystemStats {
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

    SystemStats {
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
    }
}
