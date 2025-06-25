# Thrud

A system metrics collector designed for local/small-scale network monitoring. Thrud bridges the gap between fully local performance monitoring tools (atop, htop) and large-scale frameworks (prometheus).

## Features

- **Cross-platform**: macOS (Apple Silicon), Linux, Windows (planned)
- **GPU monitoring**: Apple Silicon, NVIDIA, AMD, Intel (planned)
- **CPU monitoring**: Apple Silicon with core topology and raw tick count export
- **Stateless collectors**: Clean architecture with trait-based metric collection
- **Real-time monitoring**: Async demo app with periodic metric display

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

**System Metrics Demo** (GPU + CPU):
```bash
cargo run --bin thrud-demo
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

1. **Collectors**: Platform-specific metric collectors that produce timestamped data with metadata
2. **Storage**: Local SQLite database for historical data (planned)
3. **Transforms**: SQL-based metric computations and aggregations (planned)
4. **Interfaces**: HTTP endpoints and TUI applications (planned)

## Current Implementation

- ✅ Rust library with collector trait architecture
- ✅ Apple Silicon GPU collector via Swift FFI
- ✅ Apple Silicon CPU collector with raw tick count export
- ✅ Async demo application with GPU + CPU monitoring
- ✅ Cross-platform build system
- 🚧 SQLite storage layer
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
  bin/
    demo.rs            # Demo application
build.rs               # Build script for Swift compilation
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

2. Return `Metric` structs with appropriate metadata:
```rust
Metric::new(
    "metric_name".to_string(),
    MetricValue::Float(42.0),
    metadata_map,
)
```

## License

MIT