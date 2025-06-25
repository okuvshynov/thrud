use std::time::Duration;
use tokio::time;
use thrud::collectors::{GPUCollector, CPUCollector, Collector, MetricValue};

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
    // Group metrics by GPU
    let mut gpu_metrics: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
    
    for metric in metrics {
        let gpu_name = metric.metadata.get("gpu_name")
            .cloned()
            .unwrap_or_else(|| "Unknown GPU".to_string());
        
        gpu_metrics.entry(gpu_name).or_insert_with(Vec::new).push(metric);
    }
    
    for (gpu_name, metrics) in gpu_metrics {
        println!("\n📊 {}", gpu_name);
        
        // Find utilization metric
        let mut utilization = None;
        let mut other_metrics = Vec::new();
        
        for metric in metrics {
            match metric.name.as_str() {
                "gpu_utilization" => {
                    if let MetricValue::Float(f) = metric.value {
                        utilization = Some(f);
                    }
                }
                _ => other_metrics.push(metric),
            }
        }
        
        // Display utilization with visual bar
        if let Some(util) = utilization {
            let percentage = (util * 100.0) as i32;
            let bar_length = 20;
            let filled = (percentage as f32 / 100.0 * bar_length as f32) as usize;
            let bar = "█".repeat(filled) + &"░".repeat(bar_length - filled);
            println!("  🔥 Utilization: {:3}% [{}]", percentage, bar);
        } else {
            println!("  🔥 Utilization: N/A");
        }
        
        // Display other metrics if any
        for metric in other_metrics {
            let value_str = match &metric.value {
                MetricValue::Float(f) => format!("{:.2}", f),
                MetricValue::Integer(i) => i.to_string(),
                MetricValue::String(s) => s.clone(),
                MetricValue::Boolean(b) => b.to_string(),
            };
            println!("  📈 {}: {}", metric.name, value_str);
        }
    }
}

fn display_cpu_metrics(metrics: Vec<thrud::collectors::Metric>) {
    println!("\n🖥️  CPU Metrics (Raw Tick Counts)");
    
    // Separate different types of CPU metrics
    let mut tick_metrics: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
    let mut core_count_metrics = Vec::new();
    
    for metric in metrics {
        match metric.name.as_str() {
            "cpu_user_ticks" | "cpu_system_ticks" | "cpu_nice_ticks" | "cpu_idle_ticks" => {
                let core_id = metric.metadata.get("core_id").unwrap_or(&"unknown".to_string()).clone();
                tick_metrics.entry(core_id).or_insert_with(Vec::new).push(metric);
            }
            "cpu_core_count" => core_count_metrics.push(metric),
            _ => {}
        }
    }
    
    // Display core count info
    for metric in core_count_metrics {
        if let MetricValue::Integer(total_cores) = metric.value {
            let efficiency_cores = metric.metadata.get("efficiency_cores").map(|s| s.as_str()).unwrap_or("?");
            let performance_cores = metric.metadata.get("performance_cores").map(|s| s.as_str()).unwrap_or("?");
            println!("  📊 Total: {} cores (🔋 {} E-cores, ⚡ {} P-cores)", 
                total_cores, efficiency_cores, performance_cores);
        }
    }
    
    // Sort cores by ID and display tick counts for first few cores as example
    let mut core_ids: Vec<_> = tick_metrics.keys().collect();
    core_ids.sort_by(|a, b| {
        let id_a = a.parse::<i32>().unwrap_or(0);
        let id_b = b.parse::<i32>().unwrap_or(0);
        id_a.cmp(&id_b)
    });
    
    println!("  📈 Sample tick counts (per core):");
    
    for (_, core_id) in core_ids.iter().take(4).enumerate() {  // Show first 4 cores as example
        if let Some(core_metrics) = tick_metrics.get(*core_id) {
            let mut user_ticks = 0i64;
            let mut system_ticks = 0i64;
            let mut nice_ticks = 0i64;
            let mut idle_ticks = 0i64;
            let mut core_type = "?";
            
            for metric in core_metrics {
                if let MetricValue::Integer(value) = metric.value {
                    match metric.name.as_str() {
                        "cpu_user_ticks" => user_ticks = value,
                        "cpu_system_ticks" => system_ticks = value,
                        "cpu_nice_ticks" => nice_ticks = value,
                        "cpu_idle_ticks" => idle_ticks = value,
                        _ => {}
                    }
                }
                core_type = metric.metadata.get("core_type").map(|s| {
                    match s.as_str() {
                        "efficiency" => "E",
                        "performance" => "P",
                        _ => "?",
                    }
                }).unwrap_or("?");
            }
            
            let total_ticks = user_ticks + system_ticks + nice_ticks + idle_ticks;
            
            println!("    Core {}{}: user={}, sys={}, nice={}, idle={} (total={})", 
                core_type, core_id, user_ticks, system_ticks, nice_ticks, idle_ticks, total_ticks);
        }
    }
    
    if core_ids.len() > 4 {
        println!("    ... and {} more cores", core_ids.len() - 4);
    }
    
    println!("  ℹ️  Note: Tick counts are cumulative since boot. Rate calculation requires");
    println!("      maintaining state between samples for actual CPU utilization.");
}