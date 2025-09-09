# Probing - Dynamic Performance Profiler for Distributed AI

<div align="center">
  <img src="probing.svg" alt="Probing Logo" width="200"/>
</div>

> Uncover the Hidden Truth of AI Performance

Probing is a production-grade performance profiler designed specifically for distributed AI workloads. Built on dynamic probe injection technology, it delivers zero-overhead runtime introspection with SQL-queryable performance metrics and cross-node correlation analysis.

### What probing delivers...

- **Runtime Performance Visibility** - Expose execution bottlenecks in real-time without code modification
- **Distributed System Observability** - Cross-node performance correlation and bottleneck identification  
- **Production-Ready Monitoring** - Continuous profiling with <1% overhead for large-scale training jobs

### In contrast with traditional profilers, probing does not...

- **Require Code Modification** - No need to add logging, insert timers, or modify training scripts
- **Force "Break-Then-Fix" Debugging** - No waiting for issues to occur, then spending days reproducing them
- **Force You to Decode Fixed Reports** - No more deciphering pre-formatted tables where every row and column needs interpretation; use SQL to create custom analysis reports

[![PyPI version](https://badge.fury.io/py/probing.svg)](https://badge.fury.io/py/probing)
[![License](https://img.shields.io/badge/License-GPL%203.0-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Downloads](https://pepy.tech/badge/probing)](https://pepy.tech/project/probing)

## Getting Started

### Installation

```bash
pip install probing
```

### Quick Start (30 seconds)

```bash
# Enable instrumentation at startup
PROBING=1 python train.py

# Or inject into running process
probing -t <pid> inject

# Real-time stack trace analysis
probing -t <pid> backtrace
```

## Core Features

- **Dynamic Probe Injection** - Runtime instrumentation without target application modification
- **Distributed Performance Aggregation** - Cross-node data collection with unified correlation analysis
- **SQL Analytics Interface** - Apache DataFusion-powered query engine with standard SQL syntax
- **Interactive Python REPL** - Live debugging and variable inspection in running processes
- **Production-Grade Overhead** - Efficient sampling strategies maintaining <1% performance impact
- **Time-Series Storage** - Columnar data storage with configurable compression and retention
- **Real-Time Introspection** - Live performance metrics and runtime stack trace analysis
- **Advanced CLI** - Comprehensive command-line interface with process monitoring and management

## Basic Usage

```bash
# Inject performance monitoring
probing -t <pid> inject

# Real-time stack trace analysis
probing -t <pid> backtrace

# Memory usage profiling
probing -t <pid> memory

# Generate flame graphs
probing -t <pid> flamegraph

# Interactive Python REPL (connect to running process)
probing -t <pid> repl
```

## Advanced Features

### SQL Analytics Interface
```bash
# Memory usage analysis
probing -t <pid> query "SELECT * FROM memory_usage WHERE timestamp > now() - interval '5 min'"

# Performance hotspot analysis
probing -t <pid> query "
  SELECT operation_name, avg(duration_ms), count(*)
  FROM profiling_data 
  WHERE timestamp > now() - interval '5 minutes'
  GROUP BY operation_name
  ORDER BY avg(duration_ms) DESC
"

# Training progress tracking
probing -t <pid> query "
  SELECT epoch, avg(loss), min(loss), count(*) as steps
  FROM training_logs 
  GROUP BY epoch 
  ORDER BY epoch
"
```

### Interactive Python REPL

Probing provides an interactive Python REPL that connects to running processes, allowing you to inspect variables, execute code, and debug in real-time:

```bash
# Connect to a process via REPL
probing -t <pid> repl

# For remote processes
probing -t <host|ip:port> repl
```

Example REPL session:
```python
>>> import torch
>>> # Inspect torch models in the target process
>>> models = [m for m in gc.get_objects() if isinstance(m, torch.nn.Module)]
```

The REPL provides:
- **Live Variable Inspection**: Access all variables in the target process context
- **Code Execution**: Run arbitrary Python code within the target process
- **Real-time Debugging**: Set breakpoints and inspect state without stopping the process

### Distributed Training Analysis
```bash
# Monitor all cluster nodes
probing cluster attach

# Inter-node communication latency
probing -t <pid> query "SELECT src_rank, dst_rank, avg(latency_ms) FROM comm_metrics"

# Cross-node stack trace comparison
probing -t <pid> query "SELECT * FROM python.backtrace"

# GPU utilization analysis
probing -t <pid> query "SELECT avg(gpu_util) FROM gpu_metrics WHERE timestamp > now() - 60"
```

### Memory Analysis
```bash
# Quick memory usage overview
probing -t <pid> memory

# Memory growth trend analysis
probing -t <pid> query "SELECT hour(timestamp), avg(memory_mb) FROM memory_usage GROUP BY hour(timestamp)"

# Memory leak detection
probing -t <pid> query "
  SELECT function_name, sum(allocated_bytes) as total_alloc
  FROM memory_allocations 
  WHERE timestamp > now() - interval '1 hour'
  GROUP BY function_name
  ORDER BY total_alloc DESC
"
```

### Configuration Options
```bash
# Environment variable configuration
export PROBING_SAMPLE_RATE=0.1      # Set sampling rate
export PROBING_RETENTION_DAYS=7     # Data retention period

# View current configuration
probing -t <pid> config

# Dynamic configuration updates
probing -t <pid> config probing.sample_rate=0.05
probing -t <pid> config probing.max_memory=1GB
```

## Development

### Prerequisites

Before building Probing from source, ensure you have the following dependencies installed:

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain (required)
rustup toolchain install nightly
rustup default nightly

# Add WebAssembly target for web UI
rustup target add wasm32-unknown-unknown

# Install trunk for building WebAssembly frontend
cargo install trunk

# Install cross-compilation tools (optional, for distribution builds)
cargo install cargo-zigbuild
pip install ziglang
sudo apt-get install libunwind-dev
export RUSTFLAGS="-L/usr/lib/x86_64-linux-gnu"
```

### Building from Source

```bash
# Clone repository
git clone https://github.com/reiase/probing.git
cd probing

# Development build (faster compilation)
make

# Production build with cross-platform compatibility
make ZIG=1

# Build web UI separately (optional)
cd app && trunk build --release

# Build and install wheel package
make wheel
pip install dist/probing-*.whl --force-reinstall
```

### Testing

```bash
# Run all tests
make test

# Test with a simple example
PROBE=1 python examples/test_probing.py

# Advanced testing with variable tracking
PROBE_TORCH_EXPRS="loss@train,acc1@train" PROBE=1 python examples/imagenet.py
```

### Project Structure

- `probing/cli/` - Command-line interface
- `probing/core/` - Core profiling engine  
- `probing/extensions/` - Language-specific extensions (Python, C++)
- `probing/server/` - HTTP API server
- `app/` - Web UI (WebAssembly + Leptos)
- `python/` - Python hooks and integration
- `examples/` - Usage examples and demos

### Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes and add tests
4. Run tests: `make test`
5. Submit a pull request

## License

[GNU General Public License v3.0](LICENSE)
