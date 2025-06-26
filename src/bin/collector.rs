use std::time::Duration;
use tokio::time;
use thrud::collectors::{GPUCollector, CPUCollector, Collector};
use thrud::storage::{SqliteStorage, Storage};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Thrud System Metrics Collector", long_about = None)]
struct Args {
    /// Collection interval in seconds (supports fractional values, e.g., 0.1 for 100ms)
    #[arg(short, long, default_value = "5.0")]
    interval: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Validate interval
    if args.interval <= 0.0 {
        eprintln!("Error: Interval must be positive");
        std::process::exit(1);
    }
    
    println!("Thrud System Metrics Collector");
    println!("==============================");
    println!("Collection interval: {}s", args.interval);
    println!("Collecting metrics and storing to database...");
    println!("Press Ctrl+C to stop\n");

    // Initialize storage
    let storage = SqliteStorage::new(None); // Uses default ~/.thrud/thrud.db
    storage.initialize()?;
    
    println!("📁 Database initialized at ~/.thrud/thrud.db");

    let gpu_collector = GPUCollector::new();
    let cpu_collector = CPUCollector::new();
    let mut interval = time::interval(Duration::from_secs_f64(args.interval));

    // Show initial stats
    show_stats(&storage)?;
    
    let dev_mode = std::env::var("THRUD_DEV_MODE").is_ok();
    
    if dev_mode {
        println!("🔧 Development mode active");
    }
    
    if args.interval < 1.0 {
        println!("⚡ High-frequency mode: Reduced logging for subsecond intervals");
    }

    let mut collection_count = 0u64;
    
    loop {
        interval.tick().await;
        collection_count += 1;
        
        let mut all_metrics = Vec::new();
        let mut collection_errors = Vec::new();

        // Collect GPU metrics
        match gpu_collector.collect() {
            Ok(mut metrics) => {
                all_metrics.append(&mut metrics);
            }
            Err(e) => {
                collection_errors.push(format!("GPU: {}", e));
            }
        }
        
        // Collect CPU metrics
        match cpu_collector.collect() {
            Ok(mut metrics) => {
                all_metrics.append(&mut metrics);
            }
            Err(e) => {
                collection_errors.push(format!("CPU: {}", e));
            }
        }

        // Report collection errors (always show errors)
        for error in &collection_errors {
            println!("❌ Collection error: {}", error);
        }

        // Store metrics to database
        let metrics_count = all_metrics.len();
        if !all_metrics.is_empty() {
            match storage.store_metrics(all_metrics) {
                Ok(_) => {
                    // Silent success for frequent collections
                }
                Err(e) => {
                    println!("❌ Storage error: {}", e);
                }
            }
        } else if !collection_errors.is_empty() {
            println!("⚠️  No metrics to store");
        }

        // Show periodic stats based on interval
        let stats_interval = if args.interval < 1.0 {
            // For subsecond intervals, show stats every ~10 seconds
            (10.0 / args.interval) as u64
        } else {
            // For >= 1s intervals, show stats every ~30 seconds  
            std::cmp::max(1, (30.0 / args.interval) as u64)
        };
        
        if collection_count % stats_interval == 0 {
            println!("📊 Collection #{} at {}", collection_count, chrono::Utc::now().format("%H:%M:%S"));
            if dev_mode {
                println!("🔧 Dev info: {} total metrics in this cycle", metrics_count);
            }
            show_stats(&storage)?;
        }
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