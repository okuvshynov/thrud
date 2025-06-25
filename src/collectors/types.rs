use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: MetricValue,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
}

impl Metric {
    pub fn new(name: String, value: MetricValue, metadata: HashMap<String, String>) -> Self {
        Self {
            name,
            value,
            timestamp: Utc::now(),
            metadata,
        }
    }
}

pub trait Collector {
    fn collect(&self) -> Result<Vec<Metric>, Box<dyn std::error::Error>>;
    fn name(&self) -> &str;
}