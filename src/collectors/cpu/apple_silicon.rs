use crate::collectors::{Collector, Metric, MetricValue};
use std::collections::HashMap;
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
    total_cores: i32,
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

        // Export raw tick counts for each core
        for tick_data in &cpu_data.tick_counts {
            // Find matching core info for metadata
            let core_info = cpu_data.cores.iter().find(|c| c.id == tick_data.core_id);
            
            let mut metadata = HashMap::new();
            metadata.insert("core_id".to_string(), tick_data.core_id.to_string());
            
            if let Some(core_info) = core_info {
                let core_type_str = match core_info.core_type {
                    1 => "efficiency",
                    2 => "performance",
                    _ => "unknown",
                };
                metadata.insert("core_type".to_string(), core_type_str.to_string());
                metadata.insert("cluster_id".to_string(), core_info.cluster_id.to_string());
            } else {
                metadata.insert("core_type".to_string(), "unknown".to_string());
                metadata.insert("cluster_id".to_string(), "-1".to_string());
            }

            // Export individual tick counts
            metrics.push(Metric::new(
                "cpu_user_ticks".to_string(),
                MetricValue::Integer(tick_data.user_ticks as i64),
                metadata.clone(),
            ));
            
            metrics.push(Metric::new(
                "cpu_system_ticks".to_string(),
                MetricValue::Integer(tick_data.system_ticks as i64),
                metadata.clone(),
            ));
            
            metrics.push(Metric::new(
                "cpu_nice_ticks".to_string(),
                MetricValue::Integer(tick_data.nice_ticks as i64),
                metadata.clone(),
            ));
            
            metrics.push(Metric::new(
                "cpu_idle_ticks".to_string(),
                MetricValue::Integer(tick_data.idle_ticks as i64),
                metadata,
            ));
        }

        // Add core count metadata
        let mut core_count_metadata = HashMap::new();
        core_count_metadata.insert("total_cores".to_string(), cpu_data.total_cores.to_string());
        
        let efficiency_count = cpu_data.cores.iter().filter(|c| c.core_type == 1).count();
        let performance_count = cpu_data.cores.iter().filter(|c| c.core_type == 2).count();
        
        core_count_metadata.insert("efficiency_cores".to_string(), efficiency_count.to_string());
        core_count_metadata.insert("performance_cores".to_string(), performance_count.to_string());
        
        metrics.push(Metric::new(
            "cpu_core_count".to_string(),
            MetricValue::Integer(cpu_data.total_cores as i64),
            core_count_metadata,
        ));

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