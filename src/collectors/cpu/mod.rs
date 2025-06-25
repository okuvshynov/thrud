pub mod apple_silicon;

use super::{Collector, Metric};

pub struct CPUCollector {
    #[cfg(target_os = "macos")]
    apple_silicon: apple_silicon::AppleSiliconCPUCollector,
}

impl CPUCollector {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "macos")]
            apple_silicon: apple_silicon::AppleSiliconCPUCollector::new(),
        }
    }
}

impl Collector for CPUCollector {
    fn collect(&self) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        #[cfg(target_os = "macos")]
        {
            self.apple_silicon.collect()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(vec![])
        }
    }

    fn name(&self) -> &str {
        "cpu"
    }
}