# Thrud

A system metrics collector designed for local/small-scale network monitoring. Thrud bridges the gap between fully local performance monitoring tools (atop, htop) and large-scale frameworks (prometheus).

## Features

- **Cross-platform**: macOS (Apple Silicon), Linux, Windows (planned)
- **GPU monitoring**: Apple Silicon, NVIDIA, AMD, Intel (planned)
- **CPU monitoring**: Apple Silicon with core topology and hierarchical tick count export
- **Persistent Storage**: SQLite database with collection round tracking
- **Stateless collectors**: Clean architecture with trait-based metric collection
- **Real-time monitoring**: Multiple apps - stateless demo and persistent collector
- **Utilization Analysis**: Delta-based CPU/GPU utilization calculations

## Quick Start

### Prerequisites

- Rust (latest stable)
- On macOS: Xcode command line tools for Swift compilation

### Installation

```bash
git clone <repository-url>
cd thrud
cargo build
```

### Usage

**Stateless System Metrics Demo** (GPU + CPU):
```bash
cargo run --bin thrud-demo
```

**Persistent Metrics Collection** (with SQLite storage):
```bash
cargo run --bin thrud-collector
```

**Utilization Analysis** (query stored metrics):
```bash
# Show last 5 collection rounds (default)
./scripts/show_utilization.sh

# Show specific number of rounds
./scripts/show_utilization.sh 3
```

**Swift Proof-of-Concept Tools**:
```bash
# Apple Silicon CPU monitoring
swift samples/cpu_monitor.swift

# Apple Silicon GPU monitoring (one-time)
swift samples/gpu_monitor.swift --once

# GPU monitoring with custom interval
swift samples/gpu_monitor.swift --interval 2.0
```

## Architecture

Thrud follows a layered architecture:

1. **Collectors**: Platform-specific metric collectors that produce simple string-based metrics with timestamps
2. **Storage**: Local SQLite database (`~/.thrud/thrud.db`) with collection round tracking
3. **Analysis**: Shell scripts for delta-based utilization calculations
4. **Interfaces**: Demo apps, persistent collector, and analysis tools (HTTP endpoints and TUI planned)

## Current Implementation

- ✅ Rust library with collector trait architecture
- ✅ Apple Silicon GPU collector via Swift FFI
- ✅ Apple Silicon CPU collector with hierarchical tick count export
- ✅ SQLite storage layer with collection round tracking
- ✅ Stateless demo application with real-time GPU + CPU monitoring
- ✅ Persistent collector application with database storage
- ✅ Utilization analysis script with delta-based calculations
- ✅ Cross-platform build system
- 🚧 HTTP API endpoints
- 🚧 TUI interface

## Development

### Project Structure

```
src/
  lib.rs              # Main library entry
  collectors/
    mod.rs             # Collectors module
    types.rs           # Metric types and traits
    gpu/
      mod.rs           # Unified GPU collector interface
      apple_silicon.rs # Apple Silicon GPU implementation
      apple_silicon_bridge.swift  # Swift FFI bridge
    cpu/
      mod.rs           # Unified CPU collector interface
      apple_silicon.rs # Apple Silicon CPU implementation
      apple_silicon_bridge.swift  # Swift FFI bridge
  storage/
    mod.rs             # Storage trait and types
    sqlite.rs          # SQLite implementation
  bin/
    demo.rs            # Stateless demo application
    collector.rs       # Persistent collector application
build.rs               # Build script for Swift compilation
scripts/
  show_utilization.sh  # Delta-based utilization analysis
samples/               # Proof-of-concept Swift tools
  cpu_monitor.swift    # Standalone CPU monitor
  gpu_monitor.swift    # Standalone GPU monitor
```

### Adding New Collectors

1. Implement the `Collector` trait:
```rust
impl Collector for MyCollector {
    fn collect(&self) -> Result<Vec<Metric>, Box<dyn std::error::Error>> {
        // Collect metrics
    }
    
    fn name(&self) -> &str {
        "my_collector"
    }
}
```

2. Return `Metric` structs with simple string values:
```rust
Metric::new(
    "metric_name".to_string(),
    "42.0".to_string(),
)
```

### Storage and Analysis

The SQLite storage layer automatically handles:
- Database creation at `~/.thrud/thrud.db`
- Collection round tracking with UUIDs
- Atomic metric storage with timestamps

Use the utilization script to analyze stored data:
```bash
# Show recent utilization trends
./scripts/show_utilization.sh 10
```

## License

MIT