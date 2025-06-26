use thrud::storage::{SqliteStorage, ChartType};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Query pre-computed charts from Thrud database", long_about = None)]
struct Args {
    /// Chart type to retrieve
    #[arg(short, long, default_value = "bar")]
    chart_type: String,
    
    /// Number of latest charts to retrieve
    #[arg(short, long, default_value = "1")]
    limit: usize,
    
    /// Output format: compact (charts only) or verbose (with metadata)
    #[arg(short, long, default_value = "compact")]
    format: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Parse chart type
    let chart_type = match args.chart_type.as_str() {
        "bar" => ChartType::Bar,
        "braille" => ChartType::Braille,
        _ => {
            eprintln!("Error: Invalid chart type '{}'. Use 'bar' or 'braille'", args.chart_type);
            std::process::exit(1);
        }
    };
    
    // Initialize storage
    let storage = SqliteStorage::new(None);
    
    // Get charts
    let metrics = ["performance_cores_utilization", "efficiency_cores_utilization", "gpu_utilization"];
    let charts = storage.get_latest_charts(&metrics, &chart_type, args.limit)?;
    
    if charts.is_empty() {
        eprintln!("No charts found. Make sure the collector is running and has generated data.");
        std::process::exit(1);
    }
    
    // Output based on format
    match args.format.as_str() {
        "compact" => {
            // Group charts by collection round and output in the format expected by shell scripts
            let mut charts_by_round: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
            
            for chart in charts {
                charts_by_round.entry(chart.collection_round_id.clone()).or_insert_with(Vec::new).push(chart);
            }
            
            // Get the most recent round
            if let Some((_, round_charts)) = charts_by_round.iter().next() {
                let mut output = String::new();
                
                // Find charts for each metric in order
                for metric in &["performance_cores_utilization", "efficiency_cores_utilization", "gpu_utilization"] {
                    if let Some(chart) = round_charts.iter().find(|c| &c.metric_name == metric) {
                        let prefix = match *metric {
                            "performance_cores_utilization" => "P:",
                            "efficiency_cores_utilization" => "E:",
                            "gpu_utilization" => "G:",
                            _ => "",
                        };
                        output.push_str(&format!("{}{}", prefix, chart.chart_data));
                    }
                }
                
                println!("{}", output.trim_end_matches('|'));
            }
        },
        "verbose" => {
            for chart in &charts {
                println!("Collection Round: {}", chart.collection_round_id);
                println!("Metric: {}", chart.metric_name);
                println!("Chart Type: {:?}", chart.chart_type);
                println!("Data Points: {}", chart.data_points);
                println!("Timestamp: {}", chart.timestamp);
                println!("Chart: {}", chart.chart_data);
                println!("---");
            }
        },
        _ => {
            eprintln!("Error: Invalid format '{}'. Use 'compact' or 'verbose'", args.format);
            std::process::exit(1);
        }
    }
    
    Ok(())
}