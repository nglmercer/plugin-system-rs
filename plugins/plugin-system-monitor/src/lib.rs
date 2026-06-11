use plugin_system::{Plugin, PluginMetadata};

pub struct SystemMonitorPlugin {
    cpu_usage: f64,
    memory_total: u64,
    memory_used: u64,
    memory_usage: f64,
    swap_total: u64,
    swap_used: u64,
    load_avg: [f64; 3],
    uptime: u64,
    process_count: usize,
}

impl Default for SystemMonitorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitorPlugin {
    pub fn new() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_total: 0,
            memory_used: 0,
            memory_usage: 0.0,
            swap_total: 0,
            swap_used: 0,
            load_avg: [0.0, 0.0, 0.0],
            uptime: 0,
            process_count: 0,
        }
    }

    pub fn get_cpu_usage(&self) -> f64 {
        self.cpu_usage
    }

    pub fn get_memory_usage(&self) -> f64 {
        self.memory_usage
    }

    pub fn get_memory_total(&self) -> u64 {
        self.memory_total
    }

    pub fn get_memory_used(&self) -> u64 {
        self.memory_used
    }

    pub fn update_metrics(&mut self) {
        self.cpu_usage = Self::read_cpu_usage();
        let (mem_total, mem_used, swap_total, swap_used) = Self::read_memory_info();
        self.memory_total = mem_total;
        self.memory_used = mem_used;
        self.memory_usage = if mem_total > 0 {
            mem_used as f64 / mem_total as f64 * 100.0
        } else {
            0.0
        };
        self.swap_total = swap_total;
        self.swap_used = swap_used;
        self.load_avg = Self::read_load_avg();
        self.uptime = Self::read_uptime();
        self.process_count = Self::read_process_count();
    }

    fn read_cpu_usage() -> f64 {
        std::fs::read_to_string("/proc/stat")
            .ok()
            .and_then(|content| {
                let line = content.lines().next()?;
                let parts: Vec<u64> = line
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if parts.len() >= 4 {
                    let idle = parts[3];
                    let total: u64 = parts.iter().sum();
                    Some((total - idle) as f64 / total as f64 * 100.0)
                } else {
                    None
                }
            })
            .unwrap_or(0.0)
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

    fn read_process_count() -> usize {
        std::fs::read_dir("/proc")
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.file_name()
                            .to_str()
                            .map_or(false, |s| s.chars().all(|c| c.is_ascii_digit()))
                    })
                    .count()
            })
            .unwrap_or(0)
    }
}

#[plugin_system::plugin_export]
impl Plugin for SystemMonitorPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "system-monitor",
            version: "0.1.0",
            authors: ["StreamDeck Core"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &plugin_system::PluginContext) {
        log::info!("SystemMonitorPlugin loaded");
        self.update_metrics();
    }

    fn on_unload(&mut self) {
        log::info!("SystemMonitorPlugin unloading");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn interface_ids(&self) -> Vec<&'static str> {
        vec!["ResourceInfo"]
    }
}
