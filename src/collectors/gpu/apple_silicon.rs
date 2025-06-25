use crate::collectors::{Collector, Metric};
use std::ffi::CStr;
use std::os::raw::c_char;

#[cfg(target_os = "macos")]
extern "C" {
    fn collect_gpu_metrics_json() -> *const c_char;
    fn free_string(ptr: *const c_char);
}

#[derive(Debug, serde::Deserialize)]
struct GPUInfo {
    utilization: Option<f64>,
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
            if let Some(utilization) = gpu.utilization {
                metrics.push(Metric::new(
                    format!("gpu.{}.utilization", index),
                    utilization.to_string(),
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