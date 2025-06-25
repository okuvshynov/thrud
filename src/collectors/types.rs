use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: String,
    pub timestamp: DateTime<Utc>,
}

impl Metric {
    pub fn new(name: String, value: String) -> Self {
        Self {
            name,
            value,
            timestamp: Utc::now(),
        }
    }
}

pub trait Collector {
    fn collect(&self) -> Result<Vec<Metric>, Box<dyn std::error::Error>>;
    fn name(&self) -> &str;
}