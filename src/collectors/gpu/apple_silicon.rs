use crate::collectors::{Collector, Metric, MetricValue};
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::c_char;

#[cfg(target_os = "macos")]
extern "C" {
    fn collect_gpu_metrics_json() -> *const c_char;
    fn free_string(ptr: *const c_char);
}

#[derive(Debug, serde::Deserialize)]
struct GPUInfo {
    name: String,
    utilization: Option<f64>,
    temperature: Option<f64>,
}

pub struct AppleSiliconGPUCollector;

impl AppleSiliconGPUCollector {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "macos")]
    fn collect_macos(&self) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        let json_ptr = unsafe { collect_gpu_metrics_json() };
        
        if json_ptr.is_null() {
            return Ok(vec![]);
        }

        let json_str = unsafe {
            CStr::from_ptr(json_ptr).to_str()?
        };

        let gpu_infos: Vec<GPUInfo> = serde_json::from_str(json_str)?;
        
        unsafe {
            free_string(json_ptr);
        }

        let mut metrics = Vec::new();

        for (index, gpu) in gpu_infos.iter().enumerate() {
            let mut metadata = HashMap::new();
            metadata.insert("gpu_name".to_string(), gpu.name.clone());
            metadata.insert("gpu_index".to_string(), index.to_string());

            if let Some(utilization) = gpu.utilization {
                metrics.push(Metric::new(
                    "gpu_utilization".to_string(),
                    MetricValue::Float(utilization),
                    metadata.clone(),
                ));
            }

            if let Some(temperature) = gpu.temperature {
                metrics.push(Metric::new(
                    "gpu_temperature".to_string(),
                    MetricValue::Float(temperature),
                    metadata,
                ));
            }
        }

        Ok(metrics)
    }

    #[cfg(not(target_os = "macos"))]
    fn collect_other(&self) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        // Placeholder for other platforms
        Ok(vec![])
    }
}

impl Collector for AppleSiliconGPUCollector {
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
        "apple_silicon_gpu"
    }
}