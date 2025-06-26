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
- **Collector App**: `src/bin/collector.rs` - persistent metrics collection with database storage and chart generation
- **Chart Query Tool**: `src/bin/chart_query.rs` - retrieves pre-computed Unicode charts from database
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
cargo run --bin thrud-collector                    # Default 5s interval
cargo run --bin thrud-collector -- --interval 0.1  # 100ms interval

# Development installation and service management
make install start    # Install locally and start as background service
make status           # Check service status and database info
make logs             # Follow collector logs
make restart          # Rebuild and restart service
make stop clean       # Stop service and clean up installation

# Query utilization metrics from database (requires collector to be running)
./scripts/show_utilization.sh [number_of_rounds]           # Detailed tabular format
./scripts/show_utilization_chart.sh [number_of_rounds]     # Compact Unicode charts
./scripts/show_utilization_braille.sh [number_of_chars]    # Dense Braille (2x density)

# Query pre-computed charts directly from database
cargo run --bin thrud-chart-query                          # Latest bar chart
cargo run --bin thrud-chart-query -- --chart-type braille  # Latest braille chart
cargo run --bin thrud-chart-query -- --format verbose      # Detailed metadata
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

### Chart Storage System

The project includes a pre-computed chart storage system for efficient visualization:

- **Database Schema**: Charts table stores pre-formatted Unicode visualizations
- **Chart Types**: Both bar charts (▁▂▃▄▅▆▇█) and Braille patterns (⣀⣤⣶⣿)
- **Automatic Generation**: Collector generates charts after each metrics collection
- **Query Interface**: `thrud-chart-query` tool for instant chart retrieval
- **Performance**: ~100x faster than real-time shell script calculations

Chart format examples:
- Bar: `P:▁▁▁▂▂▃▃▁▁▁..11%|E:▂▃▃▂▁▁▁▁▁▁..15%|G:     ..0%`
- Braille: `P:⣀⣀⣤⣤⣀..20%|E:⣤⣶⣶⣤⣤..45%|G:        ..0%`

### Architecture Implementation

The Rust implementation follows the planned layered architecture:

1. **Collectors**: `Collector` trait with stateless `collect()` method returning simple `Metric` structs with string values
2. **Storage**: `Storage` trait with SQLite implementation for persistent metric storage with collection round tracking
3. **FFI Bridge**: Swift bridge compiled to static library for Apple Silicon hardware access
4. **Cross-platform**: Conditional compilation for platform-specific collectors
5. **Chart Engine**: Pre-computation of Unicode visualizations with delta-based calculations

## Development

The project includes a comprehensive development installation system using Make:

### Development Commands
```bash
make help         # Show all available commands
make build        # Build release binaries
make install      # Install binaries to ~/.local/bin
make start        # Start collector as background service (1s interval)
make stop         # Stop collector service
make restart      # Rebuild and restart service
make status       # Show service status, database info, recent logs
make logs         # Follow collector logs in real-time
make clean        # Remove installation and stop service
make uninstall    # Complete cleanup including database

# Foreground development modes
make dev-start    # Run collector in terminal with development logging
make dev-fast     # Run collector with 100ms intervals for testing
```

### Development Workflow
```bash
# Set up development environment
make install start

# Make code changes...
make restart      # Quick rebuild and restart

# Monitor and debug
make status       # Check if running properly
make logs         # Watch real-time logs

# Clean up when done
make stop clean
```

### Development Features
- **Local Installation**: Uses `~/.local/bin` for binaries (no system pollution)
- **Launch Agent**: macOS service integration with automatic restart
- **Development Mode**: Enhanced logging when `THRUD_DEV_MODE=1`
- **Fast Intervals**: 1-second default for development (vs 5s production)
- **Log Management**: Centralized logs in `~/.thrud/logs/`
- **Quick Iteration**: `make restart` rebuilds and restarts in seconds

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