use rusqlite::{Connection, Result};
use std::path::PathBuf;
use crate::collectors::types::{Metric, MetricValue};
use chrono::{DateTime, Utc};

pub mod aggregations;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn new(db_path: Option<PathBuf>) -> Result<Self> {
        let path = db_path.unwrap_or_else(Self::default_path);
        
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
                    Some(format!("Failed to create directory: {}", e))
                )
            })?;
        }
        
        let conn = Connection::open(&path)?;
        let storage = Self { conn };
        storage.initialize()?;
        Ok(storage)
    }
    
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let storage = Self { conn };
        storage.initialize()?;
        Ok(storage)
    }
    
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to get home directory")
            .join(".thrud")
            .join("thrud.db")
    }
    
    fn initialize(&self) -> Result<()> {
        // Create metrics table with flexible schema
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                value_type TEXT NOT NULL,
                value_int INTEGER,
                value_float REAL,
                value_text TEXT,
                value_bool INTEGER,
                metadata TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
            [],
        )?;
        
        // Create indexes for efficient querying
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_metrics_name_timestamp 
             ON metrics(name, timestamp DESC)",
            [],
        )?;
        
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_metrics_timestamp 
             ON metrics(timestamp DESC)",
            [],
        )?;
        
        Ok(())
    }
    
    pub fn insert_metrics(&mut self, metrics: &[Metric]) -> Result<()> {
        let tx = self.conn.transaction()?;
        
        for metric in metrics {
            let timestamp = metric.timestamp.timestamp_millis();
            let metadata_json = serde_json::to_string(&metric.metadata)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            
            match &metric.value {
                MetricValue::Integer(v) => {
                    tx.execute(
                        "INSERT INTO metrics (name, timestamp, value_type, value_int, metadata)
                         VALUES (?1, ?2, 'integer', ?3, ?4)",
                        (&metric.name, timestamp, v, &metadata_json),
                    )?;
                },
                MetricValue::Float(v) => {
                    tx.execute(
                        "INSERT INTO metrics (name, timestamp, value_type, value_float, metadata)
                         VALUES (?1, ?2, 'float', ?3, ?4)",
                        (&metric.name, timestamp, v, &metadata_json),
                    )?;
                },
                MetricValue::String(v) => {
                    tx.execute(
                        "INSERT INTO metrics (name, timestamp, value_type, value_text, metadata)
                         VALUES (?1, ?2, 'string', ?3, ?4)",
                        (&metric.name, timestamp, v, &metadata_json),
                    )?;
                },
                MetricValue::Boolean(v) => {
                    tx.execute(
                        "INSERT INTO metrics (name, timestamp, value_type, value_bool, metadata)
                         VALUES (?1, ?2, 'boolean', ?3, ?4)",
                        (&metric.name, timestamp, *v as i32, &metadata_json),
                    )?;
                },
            }
        }
        
        tx.commit()?;
        Ok(())
    }
    
    pub fn query_latest(&self, name: &str, limit: usize) -> Result<Vec<Metric>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, timestamp, value_type, value_int, value_float, value_text, value_bool, metadata
             FROM metrics
             WHERE name = ?1
             ORDER BY timestamp DESC
             LIMIT ?2"
        )?;
        
        let metrics = stmt.query_map([name, &limit.to_string()], |row| {
            let name: String = row.get(0)?;
            let timestamp_ms: i64 = row.get(1)?;
            let value_type: String = row.get(2)?;
            let metadata_json: String = row.get(7)?;
            
            let timestamp = DateTime::<Utc>::from_timestamp_millis(timestamp_ms)
                .ok_or_else(|| rusqlite::Error::FromSqlConversionFailure(
                    1, rusqlite::types::Type::Integer, Box::new(std::fmt::Error)
                ))?;
            
            let value = match value_type.as_str() {
                "integer" => MetricValue::Integer(row.get(3)?),
                "float" => MetricValue::Float(row.get(4)?),
                "string" => MetricValue::String(row.get(5)?),
                "boolean" => MetricValue::Boolean(row.get::<_, i32>(6)? != 0),
                _ => return Err(rusqlite::Error::FromSqlConversionFailure(
                    2, rusqlite::types::Type::Text, Box::new(std::fmt::Error)
                )),
            };
            
            let metadata = serde_json::from_str(&metadata_json)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    7, rusqlite::types::Type::Text, Box::new(e)
                ))?;
            
            Ok(Metric {
                name,
                value,
                timestamp,
                metadata,
            })
        })?;
        
        metrics.collect()
    }
    
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    #[test]
    fn test_storage_initialization() {
        let storage = Storage::new_in_memory().unwrap();
        // Verify tables were created
        let count: i64 = storage.conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='metrics'",
                [],
                |row| row.get(0)
            )
            .unwrap();
        assert_eq!(count, 1);
    }
    
    #[test]
    fn test_insert_and_query() {
        let mut storage = Storage::new_in_memory().unwrap();
        
        let mut metadata = HashMap::new();
        metadata.insert("core_id".to_string(), "0".to_string());
        
        let metric = Metric::new(
            "cpu_ticks".to_string(),
            MetricValue::Integer(12345),
            metadata,
        );
        
        storage.insert_metrics(&[metric]).unwrap();
        
        let results = storage.query_latest("cpu_ticks", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "cpu_ticks");
        
        if let MetricValue::Integer(v) = results[0].value {
            assert_eq!(v, 12345);
        } else {
            panic!("Expected integer value");
        }
    }
}