pub mod apple_silicon;

use super::{Collector, Metric};

pub struct GPUCollector {
    #[cfg(target_os = "macos")]
    apple_silicon: apple_silicon::AppleSiliconGPUCollector,
}

impl GPUCollector {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "macos")]
            apple_silicon: apple_silicon::AppleSiliconGPUCollector::new(),
        }
    }
}

impl Collector for GPUCollector {
    fn collect(&self) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        #[cfg(target_os = "macos")]
        {
            self.apple_silicon.collect()
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Placeholder for other platforms
            Ok(vec![])
        }
    }

    fn name(&self) -> &str {
        "gpu"
    }
}