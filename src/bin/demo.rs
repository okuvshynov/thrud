use std::time::Duration;
use tokio::time;
use thrud::collectors::{GPUCollector, CPUCollector, Collector};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Thrud System Metrics Demo");
    println!("========================");
    println!("Press Ctrl+C to stop\n");

    let gpu_collector = GPUCollector::new();
    let cpu_collector = CPUCollector::new();
    let mut interval = time::interval(Duration::from_secs(2));

    loop {
        interval.tick().await;
        
        println!("--- System Metrics at {} ---", chrono::Utc::now().format("%H:%M:%S"));
        
        // Collect and display GPU metrics
        match gpu_collector.collect() {
            Ok(metrics) => {
                if !metrics.is_empty() {
                    display_gpu_metrics(metrics);
                }
            }
            Err(e) => {
                eprintln!("❌ Error collecting GPU metrics: {}", e);
            }
        }
        
        // Collect and display CPU metrics
        match cpu_collector.collect() {
            Ok(metrics) => {
                if !metrics.is_empty() {
                    display_cpu_metrics(metrics);
                }
            }
            Err(e) => {
                eprintln!("❌ Error collecting CPU metrics: {}", e);
            }
        }
        
        println!();
    }
}

fn display_gpu_metrics(metrics: Vec<thrud::collectors::Metric>) {
    println!("\n📊 GPU Metrics");
    
    for metric in metrics {
        if metric.name.starts_with("gpu.") && metric.name.ends_with(".utilization") {
            // Extract GPU index from metric name like "gpu.0.utilization"
            let parts: Vec<&str> = metric.name.split('.').collect();
            if parts.len() == 3 {
                let gpu_index = parts[1];
                let utilization: f64 = metric.value.parse().unwrap_or(0.0);
                
                let percentage = (utilization * 100.0) as i32;
                let bar_length = 20;
                let filled = (percentage as f32 / 100.0 * bar_length as f32) as usize;
                let bar = "█".repeat(filled) + &"░".repeat(bar_length - filled);
                println!("  🔥 GPU {}: {:3}% [{}]", gpu_index, percentage, bar);
            }
        } else {
            println!("  📈 {}: {}", metric.name, metric.value);
        }
    }
}

fn display_cpu_metrics(metrics: Vec<thrud::collectors::Metric>) {
    println!("\n🖥️  CPU Metrics (Tick Counts)");
    
    // Group metrics by type
    let mut per_core_metrics = Vec::new();
    let mut per_cluster_metrics = Vec::new();
    let mut per_type_metrics = Vec::new();
    
    for metric in metrics {
        if metric.name.contains("_core.") {
            per_core_metrics.push(metric);
        } else if metric.name.contains("_cluster.") {
            per_cluster_metrics.push(metric);
        } else if metric.name.starts_with("cpu.efficiency.") || metric.name.starts_with("cpu.performance.") {
            per_type_metrics.push(metric);
        }
    }
    
    // Display per-type aggregations
    println!("  📊 Per Core Type:");
    for metric in per_type_metrics {
        let parts: Vec<&str> = metric.name.split('.').collect();
        if parts.len() >= 3 {
            let core_type = match parts[1] {
                "efficiency" => "🔋 E-cores",
                "performance" => "⚡ P-cores", 
                _ => parts[1],
            };
            let tick_type = parts[2].replace("_ticks", "");
            println!("    {}: {} {} ticks", core_type, metric.value, tick_type);
        }
    }
    
    // Display sample per-core metrics (first 4 cores)
    let mut shown_cores = std::collections::HashSet::new();
    per_core_metrics.sort_by(|a, b| a.name.cmp(&b.name));
    
    println!("  📈 Sample Per-Core (first 4 cores):");
    for metric in per_core_metrics.iter().take(8) {  // 8 = 4 cores × 2 tick types
        if metric.name.contains(".idle_ticks") || metric.name.contains(".total_ticks") {
            let parts: Vec<&str> = metric.name.split('.').collect();
            if parts.len() >= 4 {
                let core_type_str = match parts[1] {
                    "efficiency_core" => "E",
                    "performance_core" => "P",
                    _ => "?",
                };
                let core_id = parts[2];
                let tick_type = parts[3].replace("_ticks", "");
                
                if !shown_cores.contains(core_id) && shown_cores.len() < 4 {
                    if tick_type == "idle" {
                        shown_cores.insert(core_id.to_string());
                    }
                }
                
                if shown_cores.contains(core_id) && shown_cores.len() <= 4 {
                    println!("    Core {}{}: {} {} ticks", 
                        core_type_str, core_id, metric.value, tick_type);
                }
            }
        }
    }
    
    // Display cluster aggregations if available
    if !per_cluster_metrics.is_empty() {
        println!("  🔗 Per Cluster:");
        per_cluster_metrics.sort_by(|a, b| a.name.cmp(&b.name));
        for metric in per_cluster_metrics.iter().take(4) {  // Show first 4 cluster metrics
            let parts: Vec<&str> = metric.name.split('.').collect();
            if parts.len() >= 4 {
                let cluster_type = match parts[1] {
                    "efficiency_cluster" => "🔋 E-cluster",
                    "performance_cluster" => "⚡ P-cluster",
                    _ => parts[1],
                };
                let cluster_id = parts[2];
                let tick_type = parts[3].replace("_ticks", "");
                println!("    {} {}: {} {} ticks", 
                    cluster_type, cluster_id, metric.value, tick_type);
            }
        }
    }
    
    println!("  ℹ️  Note: Tick counts are cumulative since boot.");
}