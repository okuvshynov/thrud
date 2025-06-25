use std::time::Duration;
use tokio::time;
use thrud::collectors::{GPUCollector, Collector, MetricValue};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Thrud GPU Metrics Demo");
    println!("=====================");
    println!("Press Ctrl+C to stop\n");

    let gpu_collector = GPUCollector::new();
    let mut interval = time::interval(Duration::from_secs(2));

    loop {
        interval.tick().await;
        
        match gpu_collector.collect() {
            Ok(metrics) => {
                if metrics.is_empty() {
                    println!("No GPU metrics available");
                } else {
                    println!("--- GPU Metrics at {} ---", chrono::Utc::now().format("%H:%M:%S"));
                    
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
                        
                        // Separate metrics by type for better display
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
                    
                    println!();
                }
            }
            Err(e) => {
                eprintln!("Error collecting GPU metrics: {}", e);
            }
        }
    }
}