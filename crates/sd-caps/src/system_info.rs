//! `system-info` capability, backed by `sysinfo`.
//!
//! This is the code that used to live inside `plugin-system-monitor`. It moved
//! here because a component cannot read `/proc`, and it is the least
//! contentious of the four capabilities: read-only, no device handles, nothing
//! a plugin could damage with it.

use std::sync::Mutex;

use plugin_system::{SystemInfoProvider, SystemStats};

/// Samples the machine through `sysinfo`.
///
/// The `System` handle is retained rather than rebuilt per call: `sysinfo`
/// computes CPU usage from the delta between two refreshes, so a fresh handle
/// each time would force the sleep below on every single sample.
pub struct SysinfoProvider {
    system: Mutex<sysinfo::System>,
}

impl SysinfoProvider {
    pub fn new() -> Self {
        Self {
            system: Mutex::new(sysinfo::System::new_all()),
        }
    }
}

impl Default for SysinfoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemInfoProvider for SysinfoProvider {
    fn get_stats(&self) -> Result<SystemStats, String> {
        let mut system = self
            .system
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // Two refreshes separated by the minimum interval, or the first
        // reading of a process's CPU time is meaningless.
        system.refresh_cpu_usage();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        system.refresh_cpu_usage();

        let cpu_usage = system.global_cpu_usage().clamp(0.0, 100.0) as f64;
        let cpu_model = system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|model| !model.is_empty())
            .unwrap_or_else(|| "Unknown".to_string());
        let cpu_cores = system
            .physical_core_count()
            .unwrap_or_else(|| system.cpus().len().max(1)) as u32;

        system.refresh_memory();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let load = sysinfo::System::load_average();
        let process_count = system.processes().len() as u32;
        let thread_count: usize = system
            .processes()
            .values()
            .map(|process| process.tasks().map(|tasks| tasks.len()).unwrap_or(0))
            .sum();

        Ok(SystemStats {
            cpu_usage,
            cpu_model,
            cpu_cores,
            memory_total: system.total_memory(),
            memory_used: system.used_memory(),
            swap_total: system.total_swap(),
            swap_used: system.used_swap(),
            load_avg: [load.one, load.five, load.fifteen],
            uptime_seconds: sysinfo::System::uptime(),
            process_count,
            thread_count: thread_count as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads the real machine — asserts only on invariants that hold
    /// everywhere, not on values that vary by host.
    #[test]
    fn reports_plausible_stats_for_this_machine() {
        let stats = SysinfoProvider::new().get_stats().unwrap();

        assert!(
            (0.0..=100.0).contains(&stats.cpu_usage),
            "cpu usage out of range: {}",
            stats.cpu_usage
        );
        assert!(stats.cpu_cores >= 1, "a machine has at least one core");
        assert!(stats.memory_total > 0, "a machine has some memory");
        assert!(
            stats.memory_used <= stats.memory_total,
            "used ({}) exceeds total ({})",
            stats.memory_used,
            stats.memory_total
        );
        assert!(stats.process_count >= 1, "this test is itself a process");
    }
}
