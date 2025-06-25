use super::{CollectionRound, Storage, StorageStats};
use crate::collectors::Metric;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult, OptionalExtension};
use std::error::Error;
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub struct SqliteStorage {
    db_path: String,
}

impl SqliteStorage {
    pub fn new(db_path: Option<String>) -> Self {
        let path = db_path.unwrap_or_else(|| {
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            format!("{}/.thrud/thrud.db", home_dir)
        });
        
        Self { db_path: path }
    }

    fn ensure_db_directory(&self) -> Result<(), Box<dyn Error>> {
        let db_path = Path::new(&self.db_path);
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    fn get_connection(&self) -> SqliteResult<Connection> {
        Connection::open(&self.db_path)
    }

    fn create_tables(&self) -> Result<(), Box<dyn Error>> {
        let conn = self.get_connection()?;
        
        // Create collection_rounds table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS collection_rounds (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                metrics_count INTEGER NOT NULL
            )",
            [],
        )?;

        // Create metrics table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                collection_round_id TEXT NOT NULL,
                name TEXT NOT NULL,
                value TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY(collection_round_id) REFERENCES collection_rounds(id)
            )",
            [],
        )?;

        // Create indexes for better query performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_metrics_collection_round 
             ON metrics(collection_round_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_metrics_name 
             ON metrics(name)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_metrics_timestamp 
             ON metrics(timestamp)",
            [],
        )?;

        Ok(())
    }
}

impl Storage for SqliteStorage {
    fn initialize(&self) -> Result<(), Box<dyn Error>> {
        self.ensure_db_directory()?;
        self.create_tables()?;
        Ok(())
    }

    fn store_metrics(&self, metrics: Vec<Metric>) -> Result<CollectionRound, Box<dyn Error>> {
        if metrics.is_empty() {
            return Err("Cannot store empty metrics collection".into());
        }

        let conn = self.get_connection()?;
        let collection_id = Uuid::new_v4().to_string();
        let collection_timestamp = Utc::now();
        let metrics_count = metrics.len();

        // Start transaction
        let tx = conn.unchecked_transaction()?;

        // Insert collection round
        tx.execute(
            "INSERT INTO collection_rounds (id, timestamp, metrics_count) VALUES (?1, ?2, ?3)",
            params![collection_id, collection_timestamp.to_rfc3339(), metrics_count],
        )?;

        // Insert all metrics
        for metric in &metrics {
            tx.execute(
                "INSERT INTO metrics (collection_round_id, name, value, timestamp) 
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    collection_id,
                    metric.name,
                    metric.value,
                    metric.timestamp.to_rfc3339()
                ],
            )?;
        }

        // Commit transaction
        tx.commit()?;

        Ok(CollectionRound {
            id: collection_id,
            timestamp: collection_timestamp,
            metrics_count,
        })
    }

    fn get_stats(&self) -> Result<StorageStats, Box<dyn Error>> {
        let conn = self.get_connection()?;

        // Get total metrics count
        let total_metrics: i64 = conn.query_row(
            "SELECT COUNT(*) FROM metrics",
            [],
            |row| row.get(0),
        )?;

        // Get total collection rounds count
        let total_collection_rounds: i64 = conn.query_row(
            "SELECT COUNT(*) FROM collection_rounds",
            [],
            |row| row.get(0),
        )?;

        // Get latest collection round
        let latest_collection = conn.query_row(
            "SELECT id, timestamp, metrics_count FROM collection_rounds 
             ORDER BY timestamp DESC LIMIT 1",
            [],
            |row| {
                let id: String = row.get(0)?;
                let timestamp_str: String = row.get(1)?;
                let metrics_count: usize = row.get::<_, i64>(2)? as usize;
                
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|_e| rusqlite::Error::InvalidColumnType(0, "timestamp".to_string(), rusqlite::types::Type::Text))?
                    .with_timezone(&Utc);

                Ok(CollectionRound {
                    id,
                    timestamp,
                    metrics_count,
                })
            },
        ).optional()?;

        // Get database file size
        let database_size_bytes = std::fs::metadata(&self.db_path)
            .map(|metadata| metadata.len())
            .ok();

        Ok(StorageStats {
            total_metrics,
            total_collection_rounds,
            latest_collection,
            database_size_bytes,
        })
    }
}