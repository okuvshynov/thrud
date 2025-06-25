use rusqlite::{Connection, Result};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

pub mod apple_silicon_cpu;

/// Trait for metric aggregations
pub trait Aggregation {
    /// Name of the aggregation
    fn name(&self) -> &str;
    
    /// Execute the aggregation query
    fn execute(&self, conn: &Connection, params: &HashMap<String, String>) -> Result<AggregationResult>;
    
    /// Description of the aggregation
    fn description(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    pub name: String,
    pub data: serde_json::Value,
}

/// Registry for available aggregations
pub struct AggregationRegistry {
    aggregations: HashMap<String, Box<dyn Aggregation>>,
}

impl AggregationRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            aggregations: HashMap::new(),
        };
        
        // Register built-in aggregations
        registry.register(Box::new(apple_silicon_cpu::AppleSiliconCPU::new()));
        
        registry
    }
    
    pub fn register(&mut self, aggregation: Box<dyn Aggregation>) {
        self.aggregations.insert(aggregation.name().to_string(), aggregation);
    }
    
    pub fn execute(&self, name: &str, conn: &Connection, params: &HashMap<String, String>) -> Result<AggregationResult> {
        self.aggregations
            .get(name)
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)
            .and_then(|agg| agg.execute(conn, params))
    }
    
    pub fn list(&self) -> Vec<(&str, &str)> {
        self.aggregations
            .values()
            .map(|agg| (agg.name(), agg.description()))
            .collect()
    }
}

impl Default for AggregationRegistry {
    fn default() -> Self {
        Self::new()
    }
}