use super::{Aggregation, AggregationResult};
use rusqlite::{Connection, Result};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

pub struct AppleSiliconCPU;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPURate {
    pub core_id: i32,
    pub core_type: String,
    pub cluster_id: i32,
    pub user_rate: f64,
    pub system_rate: f64,
    pub nice_rate: f64,
    pub idle_rate: f64,
    pub total_active_rate: f64,
    pub utilization_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterAggregate {
    pub core_type: String,
    pub core_count: i32,
    pub avg_utilization: f64,
    pub total_active_ticks: f64,
    pub total_idle_ticks: f64,
}

impl AppleSiliconCPU {
    pub fn new() -> Self {
        Self
    }
}

impl Aggregation for AppleSiliconCPU {
    fn name(&self) -> &str {
        "apple_silicon_cpu"
    }
    
    fn description(&self) -> &str {
        "Calculate CPU utilization rates for Apple Silicon processors with per-core and cluster aggregations"
    }
    
    fn execute(&self, conn: &Connection, params: &HashMap<String, String>) -> Result<AggregationResult> {
        let window_seconds = params
            .get("window_seconds")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(60);
        
        // Per-core rates query
        let core_rates_query = r#"
            WITH tick_windows AS (
                SELECT 
                    name,
                    timestamp,
                    value_int as ticks,
                    json_extract(metadata, '$.core_id') as core_id,
                    json_extract(metadata, '$.core_type') as core_type,
                    json_extract(metadata, '$.cluster_id') as cluster_id,
                    LAG(value_int) OVER (
                        PARTITION BY name, json_extract(metadata, '$.core_id') 
                        ORDER BY timestamp
                    ) as prev_ticks,
                    LAG(timestamp) OVER (
                        PARTITION BY name, json_extract(metadata, '$.core_id') 
                        ORDER BY timestamp
                    ) as prev_timestamp
                FROM metrics
                WHERE name IN ('cpu_user_ticks', 'cpu_system_ticks', 'cpu_nice_ticks', 'cpu_idle_ticks')
                    AND timestamp > (strftime('%s', 'now') * 1000 - ?1 * 1000)
            ),
            tick_rates AS (
                SELECT 
                    name,
                    timestamp,
                    core_id,
                    core_type,
                    cluster_id,
                    CASE 
                        WHEN prev_ticks IS NOT NULL AND timestamp > prev_timestamp
                        THEN CAST((ticks - prev_ticks) AS REAL) / ((timestamp - prev_timestamp) / 1000.0)
                        ELSE NULL
                    END as tick_rate
                FROM tick_windows
                WHERE prev_ticks IS NOT NULL
            ),
            latest_rates AS (
                SELECT 
                    core_id,
                    core_type,
                    cluster_id,
                    MAX(timestamp) as timestamp,
                    SUM(CASE WHEN name = 'cpu_user_ticks' THEN tick_rate ELSE 0 END) as user_rate,
                    SUM(CASE WHEN name = 'cpu_system_ticks' THEN tick_rate ELSE 0 END) as system_rate,
                    SUM(CASE WHEN name = 'cpu_nice_ticks' THEN tick_rate ELSE 0 END) as nice_rate,
                    SUM(CASE WHEN name = 'cpu_idle_ticks' THEN tick_rate ELSE 0 END) as idle_rate
                FROM tick_rates
                WHERE timestamp = (SELECT MAX(timestamp) FROM tick_rates t2 WHERE t2.core_id = tick_rates.core_id)
                GROUP BY core_id, core_type, cluster_id
            )
            SELECT 
                CAST(core_id AS INTEGER) as core_id,
                core_type,
                CAST(cluster_id AS INTEGER) as cluster_id,
                user_rate,
                system_rate,
                nice_rate,
                idle_rate,
                user_rate + system_rate + nice_rate as total_active_rate,
                CASE 
                    WHEN (user_rate + system_rate + nice_rate + idle_rate) > 0
                    THEN 100.0 * (user_rate + system_rate + nice_rate) / (user_rate + system_rate + nice_rate + idle_rate)
                    ELSE 0
                END as utilization_percent
            FROM latest_rates
            ORDER BY core_id
        "#;
        
        let mut stmt = conn.prepare(core_rates_query)?;
        let core_rates: Vec<CPURate> = stmt.query_map([window_seconds], |row| {
            Ok(CPURate {
                core_id: row.get(0)?,
                core_type: row.get(1)?,
                cluster_id: row.get(2)?,
                user_rate: row.get(3)?,
                system_rate: row.get(4)?,
                nice_rate: row.get(5)?,
                idle_rate: row.get(6)?,
                total_active_rate: row.get(7)?,
                utilization_percent: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
        
        // Cluster aggregation query
        let cluster_query = r#"
            WITH tick_windows AS (
                SELECT 
                    name,
                    timestamp,
                    value_int as ticks,
                    json_extract(metadata, '$.core_id') as core_id,
                    json_extract(metadata, '$.core_type') as core_type,
                    json_extract(metadata, '$.cluster_id') as cluster_id,
                    LAG(value_int) OVER (
                        PARTITION BY name, json_extract(metadata, '$.core_id') 
                        ORDER BY timestamp
                    ) as prev_ticks,
                    LAG(timestamp) OVER (
                        PARTITION BY name, json_extract(metadata, '$.core_id') 
                        ORDER BY timestamp
                    ) as prev_timestamp
                FROM metrics
                WHERE name IN ('cpu_user_ticks', 'cpu_system_ticks', 'cpu_nice_ticks', 'cpu_idle_ticks')
                    AND timestamp > (strftime('%s', 'now') * 1000 - ?1 * 1000)
            ),
            tick_rates AS (
                SELECT 
                    name,
                    core_type,
                    core_id,
                    CASE 
                        WHEN prev_ticks IS NOT NULL AND timestamp > prev_timestamp
                        THEN CAST((ticks - prev_ticks) AS REAL) / ((timestamp - prev_timestamp) / 1000.0)
                        ELSE NULL
                    END as tick_rate,
                    CASE 
                        WHEN name IN ('cpu_user_ticks', 'cpu_system_ticks', 'cpu_nice_ticks')
                        THEN 'active'
                        ELSE 'idle'
                    END as state_type
                FROM tick_windows
                WHERE prev_ticks IS NOT NULL
            ),
            core_aggregates AS (
                SELECT 
                    core_type,
                    state_type,
                    SUM(tick_rate) as total_rate,
                    COUNT(DISTINCT core_id) as core_count
                FROM tick_rates
                GROUP BY core_type, state_type
            )
            SELECT 
                core_type,
                MAX(CASE WHEN state_type = 'active' THEN core_count ELSE 0 END) as core_count,
                CASE 
                    WHEN SUM(total_rate) > 0
                    THEN 100.0 * SUM(CASE WHEN state_type = 'active' THEN total_rate ELSE 0 END) / SUM(total_rate)
                    ELSE 0
                END as avg_utilization,
                SUM(CASE WHEN state_type = 'active' THEN total_rate ELSE 0 END) as total_active_ticks,
                SUM(CASE WHEN state_type = 'idle' THEN total_rate ELSE 0 END) as total_idle_ticks
            FROM core_aggregates
            GROUP BY core_type
        "#;
        
        let mut stmt = conn.prepare(cluster_query)?;
        let cluster_aggregates: Vec<ClusterAggregate> = stmt.query_map([window_seconds], |row| {
            Ok(ClusterAggregate {
                core_type: row.get(0)?,
                core_count: row.get(1)?,
                avg_utilization: row.get(2)?,
                total_active_ticks: row.get(3)?,
                total_idle_ticks: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
        
        let result = serde_json::json!({
            "per_core_rates": core_rates,
            "cluster_aggregates": cluster_aggregates,
            "window_seconds": window_seconds,
        });
        
        Ok(AggregationResult {
            name: self.name().to_string(),
            data: result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::collectors::types::{Metric, MetricValue};
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_cpu_rate_calculation() {
        let mut storage = Storage::new_in_memory().unwrap();
        let aggregation = AppleSiliconCPU::new();
        
        // Insert first set of tick counts
        let mut metadata = HashMap::new();
        metadata.insert("core_id".to_string(), "0".to_string());
        metadata.insert("core_type".to_string(), "efficiency".to_string());
        metadata.insert("cluster_id".to_string(), "0".to_string());
        
        let metrics1 = vec![
            Metric::new("cpu_user_ticks".to_string(), MetricValue::Integer(1000), metadata.clone()),
            Metric::new("cpu_system_ticks".to_string(), MetricValue::Integer(500), metadata.clone()),
            Metric::new("cpu_nice_ticks".to_string(), MetricValue::Integer(0), metadata.clone()),
            Metric::new("cpu_idle_ticks".to_string(), MetricValue::Integer(8500), metadata.clone()),
        ];
        storage.insert_metrics(&metrics1).unwrap();
        
        // Wait and insert second set with incremented tick counts
        thread::sleep(Duration::from_millis(100));
        
        let metrics2 = vec![
            Metric::new("cpu_user_ticks".to_string(), MetricValue::Integer(1100), metadata.clone()),
            Metric::new("cpu_system_ticks".to_string(), MetricValue::Integer(550), metadata.clone()),
            Metric::new("cpu_nice_ticks".to_string(), MetricValue::Integer(0), metadata.clone()),
            Metric::new("cpu_idle_ticks".to_string(), MetricValue::Integer(8850), metadata.clone()),
        ];
        storage.insert_metrics(&metrics2).unwrap();
        
        // Execute aggregation
        let params = HashMap::new();
        let result = aggregation.execute(&storage.conn, &params).unwrap();
        
        // Verify result structure
        assert_eq!(result.name, "apple_silicon_cpu");
        assert!(result.data["per_core_rates"].is_array());
        assert!(result.data["cluster_aggregates"].is_array());
        
        // Check that we got some rates
        let core_rates = result.data["per_core_rates"].as_array().unwrap();
        assert!(!core_rates.is_empty());
        
        // Verify the utilization is reasonable
        let rate = &core_rates[0];
        let utilization = rate["utilization_percent"].as_f64().unwrap();
        
        // With ticks going from:
        // user: 1000->1100 (+100), system: 500->550 (+50), idle: 8500->8850 (+350)
        // Total active ticks = 150, total ticks = 500
        // Expected utilization = 150/500 = 30%
        assert!(utilization > 25.0 && utilization < 35.0, "Actual utilization: {}", utilization);
    }
}