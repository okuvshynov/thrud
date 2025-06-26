# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Thrud is a system metrics collector designed for local/small-scale network monitoring. It bridges the gap between fully local performance monitoring tools (atop, htop) and large-scale frameworks (prometheus).

## Architecture

The system follows a layered architecture:

1. **Collectors**: Platform-specific and metric-specific collectors that produce raw metrics data. Collectors are stateless and export simple string-based metrics (e.g., `cpu.efficiency.idle_ticks = "12345"`).

2. **Raw Data Storage**: Collector output is written to a local SQLite database (`~/.thrud/thrud.db`) with collection round tracking.

3. **Metric Transforms**: SQL-based transformations to derive computed metrics like disk read rates, aggregated CPU load per socket/cluster, and total power consumption.

4. **Interfaces**: Multiple interfaces including HTTP endpoints and command-line TUI applications.

## Current Implementation

The project includes both proof-of-concept Swift monitors and a working Rust implementation with GPU and CPU collection:

### Rust Implementation

- **Library**: `src/lib.rs` with collectors and storage module architecture
- **GPU Collector**: `src/collectors/gpu/` with unified interface and Apple Silicon implementation
- **CPU Collector**: `src/collectors/cpu/` with Apple Silicon implementation and hierarchical tick count export
- **Storage Layer**: `src/storage/` with SQLite backend and collection round tracking
- **Demo App**: `src/bin/demo.rs` - displays GPU and CPU metrics with real-time monitoring (stateless)
- **Collector App**: `src/bin/collector.rs` - persistent metrics collection with database storage
- **Utilization Scripts**: 
  - `scripts/show_utilization.sh` - tabular delta-based utilization analysis
  - `scripts/show_utilization_chart.sh` - compact Unicode chart visualization
  - `scripts/show_utilization_braille.sh` - dense Braille visualization (2x data density)
- **Build System**: `build.rs` compiles Swift bridges to combined static library

Build and run:
```bash
# Build the project
cargo build

# Run stateless system metrics demo (GPU + CPU)
cargo run --bin thrud-demo

# Run persistent collector with database storage
cargo run --bin thrud-collector

# Query utilization metrics from database (requires collector to be running)
./scripts/show_utilization.sh [number_of_rounds]           # Detailed tabular format
./scripts/show_utilization_chart.sh [number_of_rounds]     # Compact Unicode charts
./scripts/show_utilization_braille.sh [number_of_chars]    # Dense Braille (2x density)
```

### Swift Proof-of-Concept

- `samples/cpu_monitor.swift`: Apple Silicon CPU monitoring with P-core/E-core detection via IOKit registry
- `samples/gpu_monitor.swift`: Apple Silicon GPU utilization monitoring via IOAccelerator service

Both Swift files are executable scripts:

```bash
# Run CPU monitor
swift samples/cpu_monitor.swift

# Run GPU monitor (one-time)
swift samples/gpu_monitor.swift --once
```

### Utilization Analysis

The project includes a shell script for analyzing collected metrics:

- **Delta-based Calculations**: `scripts/show_utilization.sh` calculates real-time utilization by computing deltas between consecutive collection rounds
- **Query Strategy**: Queries N+1 rounds to display N rounds with complete delta calculations
- **Metrics Provided**: 
  - Performance cores utilization: (delta_total - delta_idle) / delta_total
  - Efficiency cores utilization: (delta_total - delta_idle) / delta_total  
  - GPU utilization: instantaneous values from database
- **Cross-platform**: Compatible with macOS/Linux using portable shell commands

```bash
# Detailed tabular format (default 5 rounds)
./scripts/show_utilization.sh

# Compact Unicode chart format (default 10 points)
./scripts/show_utilization_chart.sh

# Specific number of rounds/points
./scripts/show_utilization.sh 3
./scripts/show_utilization_chart.sh 15 --verbose

# Example chart output: P:▁▁▁▂▂..4%|E:▂▃▃▂▁..8%|G:     ..0%
```

### Architecture Implementation

The Rust implementation follows the planned layered architecture:

1. **Collectors**: `Collector` trait with stateless `collect()` method returning simple `Metric` structs with string values
2. **Storage**: `Storage` trait with SQLite implementation for persistent metric storage with collection round tracking
3. **FFI Bridge**: Swift bridge compiled to static library for Apple Silicon hardware access
4. **Cross-platform**: Conditional compilation for platform-specific collectors

## Technology Stack

- **Rust**: Main implementation language with async/await support (tokio)
- **SQLite**: Database backend for persistent metric storage with collection round tracking
- **Swift**: Hardware access bridge for Apple Silicon via IOKit
- **FFI**: C-compatible interface between Rust and Swift

## Target Platforms

- macOS (current focus with Apple Silicon support)
- Linux
- Windows (future)

GPU support planned for:
- NVIDIA
- AMD  
- Intel
- Apple Silicon (current implementation)