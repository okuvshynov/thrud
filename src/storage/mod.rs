pub mod sqlite;

pub use sqlite::*;

use crate::collectors::Metric;
use chrono::{DateTime, Utc};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct CollectionRound {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub metrics_count: usize,
}

pub trait Storage {
    fn initialize(&self) -> Result<(), Box<dyn Error>>;
    fn store_metrics(&self, metrics: Vec<Metric>) -> Result<CollectionRound, Box<dyn Error>>;
    fn get_stats(&self) -> Result<StorageStats, Box<dyn Error>>;
}

#[derive(Debug)]
pub struct StorageStats {
    pub total_metrics: i64,
    pub total_collection_rounds: i64,
    pub latest_collection: Option<CollectionRound>,
    pub database_size_bytes: Option<u64>,
}