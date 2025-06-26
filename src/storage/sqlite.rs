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

        // Create charts table for pre-computed visualizations
        conn.execute(
            "CREATE TABLE IF NOT EXISTS charts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                collection_round_id TEXT NOT NULL,
                metric_name TEXT NOT NULL,
                chart_type TEXT NOT NULL,
                chart_data TEXT NOT NULL,
                data_points INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY(collection_round_id) REFERENCES collection_rounds(id)
            )",
            [],
        )?;

        // Create indexes for charts table
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_charts_collection_round 
             ON charts(collection_round_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_charts_metric_type 
             ON charts(metric_name, chart_type)",
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

impl SqliteStorage {
    /// Store pre-computed chart data
    pub fn store_chart(&self, chart: &super::Chart) -> Result<(), Box<dyn Error>> {
        let conn = self.get_connection()?;
        
        conn.execute(
            "INSERT INTO charts (collection_round_id, metric_name, chart_type, chart_data, data_points, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                chart.collection_round_id,
                chart.metric_name,
                chart.chart_type.as_str(),
                chart.chart_data,
                chart.data_points as i64,
                chart.timestamp.to_rfc3339()
            ],
        )?;
        
        Ok(())
    }

    /// Get the latest charts for specified metrics and chart types
    pub fn get_latest_charts(&self, metric_names: &[&str], chart_type: &super::ChartType, limit: usize) -> Result<Vec<super::Chart>, Box<dyn Error>> {
        let conn = self.get_connection()?;
        
        let metric_placeholders = metric_names.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT collection_round_id, metric_name, chart_type, chart_data, data_points, timestamp 
             FROM charts 
             WHERE metric_name IN ({}) AND chart_type = ?
             ORDER BY timestamp DESC 
             LIMIT ?",
            metric_placeholders
        );
        
        let mut stmt = conn.prepare(&query)?;
        let mut params = Vec::new();
        for name in metric_names {
            params.push(*name);
        }
        params.push(chart_type.as_str());
        let limit_str = limit.to_string();
        params.push(&limit_str);
        
        let chart_iter = stmt.query_map(
            rusqlite::params_from_iter(params),
            |row| {
                let collection_round_id: String = row.get(0)?;
                let metric_name: String = row.get(1)?;
                let chart_type_str: String = row.get(2)?;
                let chart_data: String = row.get(3)?;
                let data_points: i64 = row.get(4)?;
                let timestamp_str: String = row.get(5)?;
                
                let chart_type = super::ChartType::from_str(&chart_type_str)
                    .ok_or_else(|| rusqlite::Error::InvalidColumnType(2, "chart_type".to_string(), rusqlite::types::Type::Text))?;
                
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|_| rusqlite::Error::InvalidColumnType(5, "timestamp".to_string(), rusqlite::types::Type::Text))?
                    .with_timezone(&Utc);
                
                Ok(super::Chart {
                    id: None,
                    collection_round_id,
                    metric_name,
                    chart_type,
                    chart_data,
                    data_points: data_points as usize,
                    timestamp,
                })
            },
        )?;
        
        let mut charts = Vec::new();
        for chart in chart_iter {
            charts.push(chart?);
        }
        
        Ok(charts)
    }

    /// Generate utilization charts for the most recent data
    pub fn generate_and_store_charts(&self, collection_round_id: &str, data_points: usize) -> Result<(), Box<dyn Error>> {
        // Get recent utilization data for chart generation
        let utilization_data = self.get_recent_utilization_data(data_points + 1)?;
        
        if utilization_data.len() < 2 {
            return Ok(());  // Need at least 2 data points for delta calculation
        }
        
        // Generate charts for each metric type
        let metrics = ["performance_cores_utilization", "efficiency_cores_utilization", "gpu_utilization"];
        let timestamp = Utc::now();
        
        for metric_name in &metrics {
            // Extract values for this metric
            let values = self.extract_metric_values(&utilization_data, metric_name)?;
            
            if values.len() >= data_points {
                // Generate bar chart
                let bar_chart = self.generate_bar_chart(&values[..data_points], metric_name)?;
                let bar_chart_obj = super::Chart {
                    id: None,
                    collection_round_id: collection_round_id.to_string(),
                    metric_name: metric_name.to_string(),
                    chart_type: super::ChartType::Bar,
                    chart_data: bar_chart,
                    data_points,
                    timestamp,
                };
                self.store_chart(&bar_chart_obj)?;
                
                // Generate braille chart (half the data points since each char represents 2 points)
                let braille_points = (data_points + 1) / 2;
                if values.len() >= braille_points * 2 {
                    let braille_chart = self.generate_braille_chart(&values[..braille_points * 2], metric_name)?;
                    let braille_chart_obj = super::Chart {
                        id: None,
                        collection_round_id: collection_round_id.to_string(),
                        metric_name: metric_name.to_string(),
                        chart_type: super::ChartType::Braille,
                        chart_data: braille_chart,
                        data_points: braille_points,
                        timestamp,
                    };
                    self.store_chart(&braille_chart_obj)?;
                }
            }
        }
        
        Ok(())
    }

    /// Get recent utilization data (similar to shell script logic)
    fn get_recent_utilization_data(&self, rounds: usize) -> Result<Vec<UtilizationData>, Box<dyn Error>> {
        let conn = self.get_connection()?;
        
        let query = "
            SELECT 
                cr.id as round_id,
                cr.timestamp,
                m.name,
                m.value
            FROM collection_rounds cr
            JOIN metrics m ON cr.id = m.collection_round_id
            WHERE m.name IN (
                'cpu.performance.total_ticks', 'cpu.performance.idle_ticks',
                'cpu.efficiency.total_ticks', 'cpu.efficiency.idle_ticks',
                'gpu.utilization'
            )
            ORDER BY cr.timestamp DESC
            LIMIT ?";
            
        let mut stmt = conn.prepare(query)?;
        let rows = stmt.query_map([rounds * 5], |row| {
            Ok((
                row.get::<_, String>(0)?,  // round_id
                row.get::<_, String>(1)?,  // timestamp
                row.get::<_, String>(2)?,  // name
                row.get::<_, String>(3)?   // value
            ))
        })?;
        
        let mut data: std::collections::HashMap<String, UtilizationData> = std::collections::HashMap::new();
        
        for row in rows {
            let (round_id, timestamp, name, value) = row?;
            let entry = data.entry(round_id.clone()).or_insert(UtilizationData {
                round_id: round_id.clone(),
                timestamp,
                perf_total: 0,
                perf_idle: 0,
                eff_total: 0,
                eff_idle: 0,
                gpu_util: 0.0,
            });
            
            let val: i64 = value.parse().unwrap_or(0);
            match name.as_str() {
                "cpu.performance.total_ticks" => entry.perf_total = val,
                "cpu.performance.idle_ticks" => entry.perf_idle = val,
                "cpu.efficiency.total_ticks" => entry.eff_total = val,
                "cpu.efficiency.idle_ticks" => entry.eff_idle = val,
                "gpu.utilization" => entry.gpu_util = val as f64,
                _ => {}
            }
        }
        
        let mut result: Vec<UtilizationData> = data.into_values().collect();
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Most recent first
        
        Ok(result)
    }

    /// Extract metric values with delta calculations
    fn extract_metric_values(&self, data: &[UtilizationData], metric: &str) -> Result<Vec<f64>, Box<dyn Error>> {
        if data.len() < 2 {
            return Ok(Vec::new());
        }
        
        let mut values = Vec::new();
        
        for i in 1..data.len() {
            let curr = &data[i-1];  // More recent
            let prev = &data[i];    // Older
            
            let utilization = match metric {
                "performance_cores_utilization" => {
                    let delta_total = curr.perf_total - prev.perf_total;
                    let delta_idle = curr.perf_idle - prev.perf_idle;
                    if delta_total > 0 {
                        ((delta_total - delta_idle) as f64 / delta_total as f64) * 100.0
                    } else { 0.0 }
                },
                "efficiency_cores_utilization" => {
                    let delta_total = curr.eff_total - prev.eff_total;
                    let delta_idle = curr.eff_idle - prev.eff_idle;
                    if delta_total > 0 {
                        ((delta_total - delta_idle) as f64 / delta_total as f64) * 100.0
                    } else { 0.0 }
                },
                "gpu_utilization" => curr.gpu_util,
                _ => 0.0,
            };
            
            values.push(utilization);
        }
        
        Ok(values)
    }

    /// Generate bar chart string (like the shell script)
    fn generate_bar_chart(&self, values: &[f64], _metric: &str) -> Result<String, Box<dyn Error>> {
        let bar_chars = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
        
        let mut chart = String::new();
        for &value in values {
            let clamped = value.max(0.0).min(100.0);
            let index = if clamped == 0.0 { 0 } else { 
                ((clamped / 100.0 * 8.0).floor() as usize + 1).min(8) 
            };
            chart.push_str(bar_chars[index]);
        }
        
        // Add percentage
        let avg_util = values.iter().sum::<f64>() / values.len() as f64;
        let percentage = format!("..{:>2.0}%", avg_util);
        
        Ok(format!("{}{}|", chart, percentage))
    }

    /// Generate braille chart string
    fn generate_braille_chart(&self, values: &[f64], _metric: &str) -> Result<String, Box<dyn Error>> {
        let mut chart = String::new();
        
        // Process values in pairs
        for chunk in values.chunks(2) {
            let left = chunk[0];
            let right = chunk.get(1).copied().unwrap_or(0.0);
            
            if left == 0.0 && right == 0.0 {
                chart.push(' ');
            } else {
                let left_level = self.percentage_to_braille_level(left);
                let right_level = self.percentage_to_braille_level(right);
                chart.push(self.get_braille_char(left_level, right_level));
            }
        }
        
        // Add percentage
        let avg_util = values.iter().sum::<f64>() / values.len() as f64;
        let percentage = format!("..{:>2.0}%", avg_util);
        
        Ok(format!("{}{}|", chart, percentage))
    }

    fn percentage_to_braille_level(&self, percentage: f64) -> u8 {
        let clamped = percentage.max(0.0).min(100.0);
        if clamped == 0.0 { 0 } 
        else if clamped <= 25.0 { 1 }
        else if clamped <= 50.0 { 2 }
        else if clamped <= 75.0 { 3 }
        else { 4 }
    }

    fn get_braille_char(&self, left: u8, right: u8) -> char {
        // Braille pattern mapping (same as shell script)
        match (left, right) {
            (0, 0) => ' ',
            (0, 1) => '⢀', (0, 2) => '⢠', (0, 3) => '⢰', (0, 4) => '⢸',
            (1, 0) => '⣀', (1, 1) => '⣀', (1, 2) => '⣠', (1, 3) => '⣰', (1, 4) => '⣸',
            (2, 0) => '⣄', (2, 1) => '⣄', (2, 2) => '⣤', (2, 3) => '⣴', (2, 4) => '⣼',
            (3, 0) => '⣆', (3, 1) => '⣆', (3, 2) => '⣦', (3, 3) => '⣶', (3, 4) => '⣾',
            (4, 0) => '⣇', (4, 1) => '⣇', (4, 2) => '⣧', (4, 3) => '⣷', (4, 4) => '⣿',
            _ => '⣿',
        }
    }
}

#[derive(Debug, Clone)]
struct UtilizationData {
    round_id: String,
    timestamp: String,
    perf_total: i64,
    perf_idle: i64,
    eff_total: i64,
    eff_idle: i64,
    gpu_util: f64,
}