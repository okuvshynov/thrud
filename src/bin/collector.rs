use std::time::Duration;
use tokio::time;
use thrud::collectors::{GPUCollector, CPUCollector, Collector};
use thrud::storage::{SqliteStorage, Storage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Thrud System Metrics Collector");
    println!("==============================");
    println!("Collecting metrics and storing to database...");
    println!("Press Ctrl+C to stop\n");

    // Initialize storage
    let storage = SqliteStorage::new(None); // Uses default ~/.thrud/thrud.db
    storage.initialize()?;
    
    println!("📁 Database initialized at ~/.thrud/thrud.db");

    let gpu_collector = GPUCollector::new();
    let cpu_collector = CPUCollector::new();
    let mut interval = time::interval(Duration::from_secs(5)); // Collect every 5 seconds

    // Show initial stats
    show_stats(&storage)?;

    loop {
        interval.tick().await;
        
        println!("--- Collecting metrics at {} ---", chrono::Utc::now().format("%H:%M:%S"));
        
        let mut all_metrics = Vec::new();
        let mut collection_errors = Vec::new();

        // Collect GPU metrics
        match gpu_collector.collect() {
            Ok(mut metrics) => {
                println!("✅ GPU: {} metrics collected", metrics.len());
                all_metrics.append(&mut metrics);
            }
            Err(e) => {
                collection_errors.push(format!("GPU: {}", e));
            }
        }
        
        // Collect CPU metrics
        match cpu_collector.collect() {
            Ok(mut metrics) => {
                println!("✅ CPU: {} metrics collected", metrics.len());
                all_metrics.append(&mut metrics);
            }
            Err(e) => {
                collection_errors.push(format!("CPU: {}", e));
            }
        }

        // Report collection errors
        for error in &collection_errors {
            println!("❌ Collection error: {}", error);
        }

        // Store metrics to database
        if !all_metrics.is_empty() {
            match storage.store_metrics(all_metrics) {
                Ok(collection_round) => {
                    println!("💾 Stored {} metrics to database (round: {})", 
                        collection_round.metrics_count, 
                        &collection_round.id[..8]); // Show first 8 chars of UUID
                }
                Err(e) => {
                    println!("❌ Storage error: {}", e);
                }
            }
        } else {
            println!("⚠️  No metrics to store");
        }

        // Show updated stats every few collections
        if chrono::Utc::now().timestamp() % 30 == 0 { // Every ~30 seconds
            show_stats(&storage)?;
        }
        
        println!();
    }
}

fn show_stats(storage: &SqliteStorage) -> Result<(), Box<dyn std::error::Error>> {
    let stats = storage.get_stats()?;
    
    println!("📊 Database Statistics:");
    println!("  Total metrics: {}", stats.total_metrics);
    println!("  Collection rounds: {}", stats.total_collection_rounds);
    
    if let Some(size) = stats.database_size_bytes {
        println!("  Database size: {:.2} KB", size as f64 / 1024.0);
    }
    
    if let Some(latest) = &stats.latest_collection {
        println!("  Latest collection: {} ({} metrics)", 
            latest.timestamp.format("%Y-%m-%d %H:%M:%S UTC"), 
            latest.metrics_count);
    }
    
    println!();
    Ok(())
}