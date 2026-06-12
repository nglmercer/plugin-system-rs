use plugin_system::{command, CommandResult, PluginContext, PluginMetadata};
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct SystemStats {
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

pub trait SystemMonitor: Send + Sync {
    fn get_stats(&self) -> SystemStats;
    fn refresh(&mut self);
}

pub struct SystemMonitorPlugin {
    stats: SystemStats,
}

impl Default for SystemMonitorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitorPlugin {
    #[cfg(test)]
    pub(crate) fn with_stats(stats: SystemStats) -> Self {
        Self { stats }
    }

    #[cfg(test)]
    pub(crate) fn stats(&self) -> &SystemStats {
        &self.stats
    }
}

#[plugin_system::plugin_export]
impl SystemMonitorPlugin {
    pub fn new() -> Self {
        Self {
            stats: SystemStats::default(),
        }
    }

    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "system-monitor",
            version: "0.1.0",
            authors: ["StreamDeck Core"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &PluginContext) {
        log::info!("SystemMonitorPlugin loaded");
        self.refresh();
    }

    fn on_unload(&mut self) {
        log::info!("SystemMonitorPlugin unloading");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    pub fn interface_data(&self) -> Option<serde_json::Value> {
        serde_json::to_value(&self.stats).ok()
    }

    #[command("refresh")]
    fn sys_refresh(&mut self) -> CommandResult {
        self.refresh();
        Ok(serde_json::json!({"ok": true}))
    }

    fn read_cpu_times() -> Option<(u64, u64)> {
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
            Some((idle, total))
        } else {
            None
        }
    }

    fn read_cpu_usage_sample() -> f64 {
        let first = Self::read_cpu_times();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let second = Self::read_cpu_times();

        match (first, second) {
            (Some((idle1, total1)), Some((idle2, total2))) => {
                let total_delta = total2.saturating_sub(total1);
                let idle_delta = idle2.saturating_sub(idle1);

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
            .unwrap_or_else(|| "Unknown".to_string())
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
                        if let Ok(task_dir) = std::fs::read_dir(entry.path().join("task")) {
                            threads += task_dir.flatten().count();
                        }
                    }
                }
            }
        }

        (processes, threads)
    }

    fn collect_all() -> SystemStats {
        let cpu_usage = Self::read_cpu_usage_sample();
        let cpu_model = Self::read_cpu_model();
        let cpu_cores = Self::read_cpu_cores();
        let (mem_total, mem_used, swap_total, swap_used) = Self::read_memory_info();
        let load_avg = Self::read_load_avg();
        let uptime = Self::read_uptime();
        let (process_count, thread_count) = Self::read_process_count();

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
}

impl SystemMonitor for SystemMonitorPlugin {
    fn get_stats(&self) -> SystemStats {
        self.stats.clone()
    }

    fn refresh(&mut self) {
        self.stats = Self::collect_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plugin_system::Plugin;

    fn sample_stats() -> SystemStats {
        SystemStats {
            cpu_usage: 42.5,
            cpu_model: "Test CPU".to_string(),
            cpu_cores: 8,
            memory_total: 16 * 1024 * 1024 * 1024,
            memory_used: 8 * 1024 * 1024 * 1024,
            memory_usage: 50.0,
            swap_total: 2 * 1024 * 1024 * 1024,
            swap_used: 512 * 1024 * 1024,
            load_avg: [1.0, 2.0, 3.0],
            uptime: 1234,
            process_count: 120,
            thread_count: 900,
        }
    }

    #[test]
    fn metadata_and_interface_ids_are_generated() {
        let plugin = SystemMonitorPlugin::with_stats(sample_stats());

        assert_eq!(plugin.metadata().name, "system-monitor");
        assert_eq!(plugin.interface_ids(), vec!["SystemMonitor"]);
    }

    #[test]
    fn interface_data_returns_canned_stats_without_reading_proc() {
        let plugin = SystemMonitorPlugin::with_stats(sample_stats());

        let data = plugin.interface_data().unwrap();

        assert_eq!(data["cpu_usage"], 42.5);
        assert_eq!(data["cpu_model"], "Test CPU");
        assert_eq!(data["cpu_cores"], 8);
        assert_eq!(data["load_avg"][0], 1.0);
        assert_eq!(data["process_count"], 120);
        assert_eq!(plugin.stats().cpu_model, "Test CPU");
    }

    #[test]
    fn refresh_command_uses_macro_dispatch() {
        let mut plugin = SystemMonitorPlugin::with_stats(sample_stats());

        let refreshed = plugin
            .handle_command("refresh", serde_json::json!({}))
            .unwrap();

        assert_eq!(refreshed["ok"], true);
    }
}
