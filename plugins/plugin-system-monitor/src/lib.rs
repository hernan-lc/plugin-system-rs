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

#[derive(Debug, Clone)]
struct SystemSnapshot {
    cpu_usage: f64,
    cpu_model: String,
    cpu_cores: usize,
    memory_total: u64,
    memory_used: u64,
    swap_total: u64,
    swap_used: u64,
    load_avg: [f64; 3],
    uptime: u64,
    process_count: usize,
    thread_count: usize,
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

    fn from_snapshot(snapshot: SystemSnapshot) -> SystemStats {
        let memory_usage = if snapshot.memory_total > 0 {
            snapshot.memory_used as f64 / snapshot.memory_total as f64 * 100.0
        } else {
            0.0
        };

        SystemStats {
            cpu_usage: snapshot.cpu_usage,
            cpu_model: snapshot.cpu_model,
            cpu_cores: snapshot.cpu_cores,
            memory_total: snapshot.memory_total,
            memory_used: snapshot.memory_used,
            memory_usage,
            swap_total: snapshot.swap_total,
            swap_used: snapshot.swap_used,
            load_avg: snapshot.load_avg,
            uptime: snapshot.uptime,
            process_count: snapshot.process_count,
            thread_count: snapshot.thread_count,
        }
    }
}

#[cfg(test)]
fn cpu_usage_from_deltas(idle1: u64, total1: u64, idle2: u64, total2: u64) -> f64 {
    let total_delta = total2.saturating_sub(total1);
    let idle_delta = idle2.saturating_sub(idle1);

    if total_delta > 0 {
        ((total_delta - idle_delta) as f64 / total_delta as f64 * 100.0).min(100.0)
    } else {
        0.0
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

    pub fn interface_data(&self) -> Option<serde_json::Value> {
        serde_json::to_value(&self.stats).ok()
    }

    #[command("refresh")]
    fn sys_refresh(&mut self) -> CommandResult {
        self.refresh();
        Ok(serde_json::json!({"ok": true}))
    }

    fn collect_all() -> SystemStats {
        let mut system = sysinfo::System::new_all();

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
        let cpu_cores =
            sysinfo::System::physical_core_count().unwrap_or_else(|| system.cpus().len().max(1));

        system.refresh_memory();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let load_average = sysinfo::System::load_average();
        let load_avg = [load_average.one, load_average.five, load_average.fifteen];
        let process_count = system.processes().len();
        let thread_count = system
            .processes()
            .values()
            .map(|process| process.tasks().map(|tasks| tasks.len()).unwrap_or(0))
            .sum();

        Self::from_snapshot(SystemSnapshot {
            cpu_usage,
            cpu_model,
            cpu_cores,
            memory_total: system.total_memory(),
            memory_used: system.used_memory(),
            swap_total: system.total_swap(),
            swap_used: system.used_swap(),
            load_avg,
            uptime: sysinfo::System::uptime(),
            process_count,
            thread_count,
        })
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
    fn interface_data_returns_canned_stats_without_reading_system() {
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

    #[test]
    fn cpu_usage_uses_idle_delta_ratio() {
        let usage = super::cpu_usage_from_deltas(400, 1000, 1100, 2000);

        assert_eq!(usage, 30.0);
    }

    #[test]
    fn cpu_usage_returns_zero_without_positive_total_delta() {
        let usage = super::cpu_usage_from_deltas(400, 1000, 400, 1000);

        assert_eq!(usage, 0.0);
    }

    #[test]
    fn snapshot_to_stats_calculates_memory_percentage() {
        let stats = SystemMonitorPlugin::from_snapshot(SystemSnapshot {
            cpu_usage: 12.5,
            cpu_model: "Snapshot CPU".to_string(),
            cpu_cores: 4,
            memory_total: 1000,
            memory_used: 250,
            swap_total: 500,
            swap_used: 100,
            load_avg: [0.5, 0.75, 1.0],
            uptime: 3600,
            process_count: 42,
            thread_count: 300,
        });

        assert_eq!(stats.memory_usage, 25.0);
        assert_eq!(stats.memory_used, 250);
        assert_eq!(stats.swap_used, 100);
        assert_eq!(stats.uptime, 3600);
    }
}
