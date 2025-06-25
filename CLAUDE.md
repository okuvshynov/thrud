# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Thrud is a system metrics collector designed for local/small-scale network monitoring. It bridges the gap between fully local performance monitoring tools (atop, htop) and large-scale frameworks (prometheus).

## Architecture

The system follows a layered architecture:

1. **Collectors**: Platform-specific and metric-specific collectors that produce raw metrics data. Collectors are stateless and export values with metadata (e.g., `cpu_load = {'value': 0.45, 'timestamp': 123321, 'metadata': {'core': 1, 'core_type': 'efficiency'}}`).

2. **Raw Data Storage**: Collector output is written to a local SQLite database (`~/.thrud/thrud.db`).

3. **Metric Transforms**: SQL-based transformations to derive computed metrics like disk read rates, aggregated CPU load per socket/cluster, and total power consumption.

4. **Interfaces**: Multiple interfaces including HTTP endpoints and command-line TUI applications.

## Current Implementation

The project includes both proof-of-concept Swift monitors and a working Rust implementation with GPU collection:

### Rust Implementation

- **Library**: `src/lib.rs` with collectors module architecture
- **GPU Collector**: `src/collectors/gpu.rs` with Swift FFI bindings for Apple Silicon
- **Demo App**: `src/bin/demo.rs` - periodically displays GPU metrics
- **Build System**: `build.rs` compiles Swift bridge to static library

Build and run:
```bash
# Build the project
cargo build

# Run GPU metrics demo
cargo run --bin thrud-demo
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

### Architecture Implementation

The Rust implementation follows the planned layered architecture:

1. **Collectors**: `Collector` trait with stateless `collect()` method returning `Metric` structs
2. **FFI Bridge**: Swift bridge compiled to static library for Apple Silicon hardware access
3. **Cross-platform**: Conditional compilation for platform-specific collectors

## Technology Stack

- **Rust**: Main implementation language with async/await support (tokio)
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