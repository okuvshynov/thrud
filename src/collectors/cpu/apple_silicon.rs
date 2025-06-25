use crate::collectors::{Collector, Metric};
use std::ffi::CStr;
use std::os::raw::c_char;

#[cfg(target_os = "macos")]
extern "C" {
    fn collect_cpu_metrics_json() -> *const c_char;
    fn free_string(ptr: *const c_char);
}

#[derive(Debug, serde::Deserialize)]
struct CoreInfo {
    id: i32,
    #[serde(rename = "type")]
    core_type: i32,
    cluster_id: i32,
}

#[derive(Debug, serde::Deserialize)]
struct CoreTickCounts {
    core_id: i32,
    user_ticks: i32,
    system_ticks: i32,
    nice_ticks: i32,
    idle_ticks: i32,
}

#[derive(Debug, serde::Deserialize)]
struct CPUMetricsData {
    cores: Vec<CoreInfo>,
    tick_counts: Vec<CoreTickCounts>,
}

pub struct AppleSiliconCPUCollector;

impl AppleSiliconCPUCollector {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "macos")]
    fn collect_macos(&self) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        let json_ptr = unsafe { collect_cpu_metrics_json() };
        
        if json_ptr.is_null() {
            return Ok(vec![]);
        }

        let json_str = unsafe {
            CStr::from_ptr(json_ptr).to_str()?
        };

        let cpu_data: CPUMetricsData = serde_json::from_str(json_str)?;
        
        unsafe {
            free_string(json_ptr);
        }

        let mut metrics = Vec::new();

        // Calculate per-core type and per-cluster aggregations
        let mut efficiency_idle = 0i64;
        let mut efficiency_total = 0i64;
        let mut performance_idle = 0i64;
        let mut performance_total = 0i64;
        
        // Map cluster_id -> (idle_ticks, total_ticks) for efficiency and performance cores
        let mut efficiency_clusters: std::collections::HashMap<i32, (i64, i64)> = std::collections::HashMap::new();
        let mut performance_clusters: std::collections::HashMap<i32, (i64, i64)> = std::collections::HashMap::new();

        // Export raw tick counts for each core and aggregate
        for tick_data in &cpu_data.tick_counts {
            let core_info = cpu_data.cores.iter().find(|c| c.id == tick_data.core_id);
            
            let idle_ticks = tick_data.idle_ticks as i64;
            let total_ticks = (tick_data.user_ticks + tick_data.system_ticks + tick_data.nice_ticks + tick_data.idle_ticks) as i64;
            
            if let Some(core_info) = core_info {
                match core_info.core_type {
                    1 => { // efficiency core
                        metrics.push(Metric::new(
                            format!("cpu.efficiency_core.{}.idle_ticks", tick_data.core_id),
                            idle_ticks.to_string(),
                        ));
                        metrics.push(Metric::new(
                            format!("cpu.efficiency_core.{}.total_ticks", tick_data.core_id),
                            total_ticks.to_string(),
                        ));
                        
                        efficiency_idle += idle_ticks;
                        efficiency_total += total_ticks;
                        
                        let cluster_entry = efficiency_clusters.entry(core_info.cluster_id).or_insert((0, 0));
                        cluster_entry.0 += idle_ticks;
                        cluster_entry.1 += total_ticks;
                    },
                    2 => { // performance core
                        metrics.push(Metric::new(
                            format!("cpu.performance_core.{}.idle_ticks", tick_data.core_id),
                            idle_ticks.to_string(),
                        ));
                        metrics.push(Metric::new(
                            format!("cpu.performance_core.{}.total_ticks", tick_data.core_id),
                            total_ticks.to_string(),
                        ));
                        
                        performance_idle += idle_ticks;
                        performance_total += total_ticks;
                        
                        let cluster_entry = performance_clusters.entry(core_info.cluster_id).or_insert((0, 0));
                        cluster_entry.0 += idle_ticks;
                        cluster_entry.1 += total_ticks;
                    },
                    _ => {
                        // unknown core type, still export individual metrics
                        metrics.push(Metric::new(
                            format!("cpu.unknown_core.{}.idle_ticks", tick_data.core_id),
                            idle_ticks.to_string(),
                        ));
                        metrics.push(Metric::new(
                            format!("cpu.unknown_core.{}.total_ticks", tick_data.core_id),
                            total_ticks.to_string(),
                        ));
                    }
                }
            }
        }

        // Add per-core-type aggregations
        metrics.push(Metric::new(
            "cpu.efficiency.idle_ticks".to_string(),
            efficiency_idle.to_string(),
        ));
        metrics.push(Metric::new(
            "cpu.efficiency.total_ticks".to_string(),
            efficiency_total.to_string(),
        ));
        
        metrics.push(Metric::new(
            "cpu.performance.idle_ticks".to_string(),
            performance_idle.to_string(),
        ));
        metrics.push(Metric::new(
            "cpu.performance.total_ticks".to_string(),
            performance_total.to_string(),
        ));

        // Add per-cluster aggregations
        for (cluster_id, (idle, total)) in efficiency_clusters {
            metrics.push(Metric::new(
                format!("cpu.efficiency_cluster.{}.idle_ticks", cluster_id),
                idle.to_string(),
            ));
            metrics.push(Metric::new(
                format!("cpu.efficiency_cluster.{}.total_ticks", cluster_id),
                total.to_string(),
            ));
        }
        
        for (cluster_id, (idle, total)) in performance_clusters {
            metrics.push(Metric::new(
                format!("cpu.performance_cluster.{}.idle_ticks", cluster_id),
                idle.to_string(),
            ));
            metrics.push(Metric::new(
                format!("cpu.performance_cluster.{}.total_ticks", cluster_id),
                total.to_string(),
            ));
        }

        Ok(metrics)
    }

    #[cfg(not(target_os = "macos"))]
    fn collect_other(&self) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
}

impl Collector for AppleSiliconCPUCollector {
    fn collect(&self) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        #[cfg(target_os = "macos")]
        {
            self.collect_macos()
        }
        #[cfg(not(target_os = "macos"))]
        {
            self.collect_other()
        }
    }

    fn name(&self) -> &str {
        "apple_silicon_cpu"
    }
}