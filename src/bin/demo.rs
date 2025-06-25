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
                        println!("\n{}", gpu_name);
                        
                        for metric in metrics {
                            let value_str = match &metric.value {
                                MetricValue::Float(f) => {
                                    if metric.name == "gpu_utilization" {
                                        format!("{:.1}%", f * 100.0)
                                    } else if metric.name == "gpu_temperature" {
                                        format!("{:.1}°C", f)
                                    } else {
                                        format!("{:.2}", f)
                                    }
                                }
                                MetricValue::Integer(i) => i.to_string(),
                                MetricValue::String(s) => s.clone(),
                                MetricValue::Boolean(b) => b.to_string(),
                            };
                            
                            let metric_display_name = match metric.name.as_str() {
                                "gpu_utilization" => "Utilization",
                                "gpu_temperature" => "Temperature",
                                _ => &metric.name,
                            };
                            
                            println!("  {}: {}", metric_display_name, value_str);
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