# Introduction

Welcome to **Probing** - a dynamic performance profiler and stability diagnostic tool designed specifically for AI applications. Probing addresses the unique challenges of debugging and optimizing large-scale, distributed, long-running AI workloads such as LLM training and inference.

## What is Probing?

Probing is a runtime analysis tool that injects probes into running Python processes to collect detailed performance data and enable real-time monitoring without requiring code modifications or process restarts. Think of it as a powerful magnifying glass for your AI applications that reveals exactly what's happening under the hood.

## Core Principles

Probing is built on three fundamental principles:

### 🔍 **Zero Intrusion**
- No code modifications required
- No environment setup changes needed
- No workflow disruptions
- Dynamic probe injection into running processes

### 🎯 **Zero Learning Curve** 
- Standard SQL interface for data analysis
- Familiar database query patterns
- Intuitive command-line tools
- Web-based dashboard for visualization

### 📦 **Zero Deployment Burden**
- Single binary deployment (Rust-based)
- Static compilation with minimal dependencies
- Cross-platform compatibility
- Elastic scaling capabilities

## Key Features

### 🎯 **Live Code Execution (`eval`)**
- **Arbitrary Python Code**: Run any Python code inside your target process
- **Real-time State Inspection**: Check variables, objects, and system state instantly  
- **Dynamic Behavior Modification**: Change configuration, clear caches, or trigger actions
- **Custom Metrics Collection**: Gather any data your application can compute

### 📊 **SQL-Based Analytics (`query`)**
- **Structured Performance Data**: Query PyTorch traces, call stacks, and system metrics
- **Familiar SQL Interface**: Use standard SQL syntax with powerful time-series functions
- **Real Tables, Real Data**: Query actual collected data, not synthetic examples
- **Cross-correlation Analysis**: Understand relationships between performance metrics

### 🔍 **Deep Stack Analysis (`backtrace`)**
- **Real-time Call Stack Capture**: Get live execution context with variable values from the main thread
- **Python Variable Inspection**: See current local variables, function arguments, and object state
- **Live Stack Analysis**: Query current execution state of the main thread
- **Main Thread Focus**: Analyze the primary execution path and main thread activity

### 🚀 **Production-Ready Foundation**
- **Zero-Intrusion Injection**: Attach to running processes without code changes
- **Minimal Performance Impact**: < 5% overhead in most real-world scenarios
- **Cross-Platform Support**: Works on Linux, macOS, and Windows
- **Distributed System Ready**: Monitor processes across multiple machines

## Three Core Capabilities

Probing provides three powerful capabilities that work together to give you complete insight into your running applications:

> **📝 Note**: All examples below use `$ENDPOINT` to represent your target process. Set this to your process ID (e.g., `export ENDPOINT=12345`) or remote address (e.g., `export ENDPOINT=host:port`). See the [Quick Start](#quick-start-your-first-5-minutes) section for detailed setup instructions.

### 🎯 **eval**: Execute Code in Live Processes
Run arbitrary Python code directly inside your target process to inspect state, modify behavior, or gather custom metrics.

```bash
# Check training threads
probing $ENDPOINT eval "import threading; [print(f'{t.name}: {t.is_alive()}') for t in threading.enumerate()]"
# Expected output:
# MainThread: True
# Thread-1 (train_worker): True
# Thread-2 (data_loader): True

# Check GPU memory usage
probing $ENDPOINT eval "import torch; print(f'GPU: {torch.cuda.memory_allocated()/1024**3:.1f}GB allocated, {torch.cuda.memory_reserved()/1024**3:.1f}GB cached') if torch.cuda.is_available() else print('CUDA unavailable')"
# Expected output (with GPU):
# GPU: 2.4GB allocated, 3.2GB cached
# Expected output (without GPU):
# CUDA unavailable
```

### 📊 **query**: Analyze Data with SQL
Query structured performance data using familiar SQL syntax to identify patterns, bottlenecks, and trends.

```bash
# Analyze PyTorch training patterns
probing $ENDPOINT query "
SELECT 
    step,
    module,
    SUM(allocated) as total_memory_mb,
    COUNT(*) as operation_count
FROM python.torch_trace 
WHERE step > 100 
GROUP BY step, module 
ORDER BY total_memory_mb DESC 
LIMIT 10"
# Expected output:
# step  | module           | total_memory_mb | operation_count
# ------|------------------|-----------------|----------------
# 150   | transformer.attn | 2048.5         | 24
# 149   | transformer.attn | 2047.8         | 24  
# 151   | transformer.mlp  | 1536.2         | 16
# 150   | transformer.mlp  | 1535.9         | 16
# ...
```

### 🔍 **backtrace**: Debug with Stack Context
Capture detailed call stacks with Python variable values to understand exactly what your main thread is doing at any moment.

```bash
# Capture current call stack from the main thread
probing $ENDPOINT backtrace

# Query the live main thread stack trace with variable inspection
probing $ENDPOINT query "SELECT func, file, lineno FROM python.backtrace ORDER BY depth LIMIT 3"
# Expected output:
# func           | file                  | lineno
# ---------------|-----------------------|--------
# forward_pass   | /app/training_loop.py | 89
# transformer    | /app/model.py         | 245
# multi_head_attn| /app/attention.py     | 156

# The python.backtrace table shows main thread execution context:
# - func: function name
# - file: source file path  
# - lineno: line number
# - depth: call stack depth (0 = deepest frame)
```

---

## Real-World Debugging Scenarios

Now that you understand the three core capabilities, let's see how they work together to solve common AI/ML debugging challenges:

### Scenario 1: Training Process Hanging
**Problem**: PyTorch training suddenly stops progressing.  
**Solution**: Use the three capabilities in sequence:

```bash
# 1. BACKTRACE: See what main thread is doing right now
probing $ENDPOINT backtrace

# 2. EVAL: Check broader system state  
probing $ENDPOINT eval "import threading; [(t.name, t.is_alive(), t.daemon) for t in threading.enumerate()]"
# Expected output:
# [('MainThread', True, False), ('Thread-1', False, False), ('Thread-2', True, True)]

# 3. QUERY: Analyze the captured stack context
probing $ENDPOINT query "SELECT func, file, lineno, depth FROM python.backtrace ORDER BY depth LIMIT 10"
# Expected output: (shows top 10 stack frames)
# func         | file              | lineno | depth
# -------------|-------------------|--------|-------
# wait_for_data| /app/dataloader.py| 234    | 0
# get_batch    | /app/training.py  | 89     | 1  
# train_step   | /app/main.py      | 156    | 2
# ... (up to 10 frames total)
```

### Scenario 2: Memory Leak Investigation  
**Problem**: Memory usage keeps growing during training.  
**Solution**: Monitor, analyze, then correlate:

```bash
# EVAL: Force cleanup and get current state
probing $ENDPOINT eval "import gc, torch; gc.collect(); torch.cuda.empty_cache() if torch.cuda.is_available() else None; print('Cleanup complete')"
# Expected output:
# Cleanup complete

# QUERY: Analyze allocation trends over recent training steps
probing $ENDPOINT query "SELECT step, AVG(allocated) as avg_memory, MAX(allocated) as peak_memory FROM python.torch_trace WHERE step >= (SELECT MAX(step) - 20 FROM python.torch_trace) GROUP BY step ORDER BY step"
# Expected output:
# step | avg_memory | peak_memory
# -----|------------|-------------
# 980  | 1024.5     | 1156.8
# 981  | 1026.2     | 1158.9  
# 982  | 1028.7     | 1161.4   # ← Memory growing!
# 983  | 1031.1     | 1164.2
# ...
```

### Scenario 3: Performance Bottleneck Analysis
**Problem**: Need to identify which model components are slowest.  
**Solution**: Real-time profiling with contextual analysis:

```bash
# BACKTRACE: Capture execution state during slow periods
probing $ENDPOINT backtrace

# QUERY: Find most expensive operations across recent steps  
probing $ENDPOINT query "SELECT module, stage, AVG(allocated) as avg_memory, COUNT(*) as frequency FROM python.torch_trace WHERE step >= (SELECT MAX(step) - 5 FROM python.torch_trace) GROUP BY module, stage ORDER BY avg_memory DESC LIMIT 10"
# Expected output:
# module                | stage    | avg_memory | frequency
# ---------------------|----------|------------|----------
# transformer.self_attn| forward  | 2048.7     | 25
# transformer.mlp      | forward  | 1536.4     | 20
# embedding_layer      | forward  | 512.8      | 15
# layer_norm           | forward  | 64.2       | 40
# ...

# EVAL: Get real-time system metrics for correlation
probing $ENDPOINT eval "import psutil; proc = psutil.Process(); print(f'CPU: {proc.cpu_percent():.1f}%, Memory: {proc.memory_info().rss/1024**3:.2f}GB, Threads: {proc.num_threads()}')"
# Expected output:
# CPU: 85.3%, Memory: 12.45GB, Threads: 8
````

## Quick Start: Your First 5 Minutes

Get immediate value from Probing with this streamlined workflow:

### Step 1: Set Your Target Process

All Probing commands need a target endpoint. Set `$ENDPOINT` to either a local process ID or remote address:

```bash
# Local process - find and set your Python process ID
export ENDPOINT=$(pgrep -f "python.*your_script")
# Expected result: ENDPOINT now contains your process ID (e.g., 12345)

# Or for remote processes
export ENDPOINT=remote-host:8080
# Expected result: ENDPOINT set to remote address
```

> **💡 Need help finding processes?** Use `ps aux | grep python` or `pgrep -f "python.*train"` to locate your target.

### Step 2: Connect and Explore (30 seconds)

```bash
# Connect to your process
probing $ENDPOINT inject
# Expected output:
# Successfully injected probes into process 12345

# Get basic process info
probing $ENDPOINT eval "import os, psutil; proc = psutil.Process(); print(f'PID: {os.getpid()}, Memory: {proc.memory_info().rss/1024**2:.1f}MB')"
# Expected output:
# PID: 12345, Memory: 1250.4MB
```

### Step 3: Try All Three Core Capabilities (2 minutes)

**📊 Query structured data:**
```bash
probing $ENDPOINT query "SELECT name, value FROM information_schema.df_settings LIMIT 5"
# Expected output:
# name                    | value
# ------------------------|--------
# max_threads             | 8
# memory_pool_size        | 2048
# cache_enabled           | true
# debug_mode              | false
# profiling_interval      | 100
```

**🎯 Execute live code:**
```bash
probing $ENDPOINT eval "import torch; print(f'CUDA: {torch.cuda.is_available()}') if 'torch' in globals() else print('PyTorch not loaded')"
# Expected output (with PyTorch):
# CUDA: True
# Expected output (without PyTorch):
# PyTorch not loaded
```

**🔍 Capture execution context:**
```bash
probing $ENDPOINT backtrace

probing $ENDPOINT query "SELECT func, file, lineno FROM python.backtrace ORDER BY depth LIMIT 5"
# Expected output:
# func           | file                  | lineno
# ---------------|-----------------------|--------
# forward        | /app/model.py         | 89
# train_step     | /app/training.py      | 156  
# main_loop      | /app/main.py          | 234
# ...
```

### Step 4: Real Debugging Workflow (2 minutes)

Combine all capabilities to debug like a pro:
```bash
# 1. Capture current state
probing $ENDPOINT backtrace

# 2. Check what's happening now
probing $ENDPOINT eval "import threading; print('Active threads:', len(threading.enumerate()))"
# Expected output:
# Active threads: 5

# 3. Analyze the results
probing $ENDPOINT query "SELECT func, file, lineno FROM python.backtrace ORDER BY depth"
# Expected output:
# func                | file                    | lineno
# --------------------|-------------------------|--------
# compute_loss        | /app/loss.py           | 45
# forward_pass        | /app/model.py          | 123
# train_batch         | /app/training.py       | 89
# main_loop           | /app/main.py           | 234
# <module>            | /app/main.py           | 15
```

**🎉 That's it!** You're now using all three core capabilities. Continue reading for advanced techniques and real-world scenarios.

---

## Detailed Setup Guide

For production deployments and distributed setups, here's the complete configuration:

### Local Process Discovery
```bash
# Find your training process
ps aux | grep python | grep train
# Expected output:
# user    12345  15.2  8.5 2048576 1048576 ?  S   10:30   0:05 python train_model.py
# user    12346   2.1  1.2  524288  131072 ?  S   10:31   0:01 python data_loader.py

# Or use pgrep for specific patterns
pgrep -f "python.*train"
# Expected output:
# 12345
# 12346
```

### Remote Process Setup
For distributed setups, target processes need network server enabled:
```bash
# On the remote machine - start your Python process with remote server
PROBING_PORT=8080 python your_training_script.py
# Expected output:
# Probing server listening on 0.0.0.0:8080
# Starting training...

# From your local machine - connect directly to the remote process
export ENDPOINT=remote-host:8080
# Expected result: Commands now target the remote process
```

### Process Launch Options
```bash
# Option A: Launch your application with probing enabled
PROBE=1 python your_app.py
# Expected output:
# Probing enabled for process 12345
# Your application output follows...

# Option B: Attach to an already running process (recommended)
probing $ENDPOINT inject
# Expected output:
# Successfully injected probes into process 12345
```

---

## Who Should Use Probing?

Based on the capabilities and scenarios above, Probing is designed for different roles in AI/ML teams:

### AI/ML Engineers
**Debug training instabilities and optimize model performance**
- "Why did my training suddenly diverge at step 15,000?" → Use `backtrace` + `query` to see exact main thread state
- "Which layer is using the most GPU memory?" → Use `eval` to inspect torch memory + `query` torch_trace  
- "Is my data loader causing bottlenecks?" → Use `eval` to check thread states and `backtrace` for main thread analysis

### DevOps Engineers  
**Monitor production AI services and troubleshoot issues**
- "Service is using 90% CPU but I can't reproduce it" → Use `inject` + `eval` to inspect live production
- "Memory usage keeps growing, is it a leak?" → Use `eval` + `query` for memory trend analysis
- "Which requests are taking the longest?" → Use `backtrace` to capture current main thread execution state of slow requests

### Research Scientists
**Profile experimental models and analyze performance**
- "How does my new attention mechanism compare?" → Use `query` to analyze torch_trace data
- "Are my optimizations actually faster?" → Use `eval` for before/after benchmarks
- "What's the memory footprint of different model sizes?" → Use `query` for memory analytics

### Platform Engineers
**Build monitoring infrastructure and optimize resource allocation**
- "Need custom metrics for our ML platform" → Use `eval` to collect any Python data
- "Want to track training progress across the fleet" → Use `query` for aggregated analytics
- "Need to debug distributed training issues" → Use `backtrace` to inspect live main thread execution state, combined with `eval` for multi-thread analysis

---

## How Probing Works: Architecture Overview

Understanding Probing's architecture helps you use it more effectively:

### Data Plane (Probes)
- Lightweight probing components injected into target processes
- Collect performance metrics and system data
- Minimal performance overhead (<5% in most cases)  
- Distributed architecture with no single point of failure

### Control Plane (Interface)
- Command-line tools for scripting and automation (what you've been using above)
- Web UI for visualization and dashboard creation
- REST API for integration with other tools
- SQL query engine for data analysis

This architecture enables the zero-intrusion, zero-setup experience you've seen in the examples above.

---

## Comparison with Other Tools

| Feature | Probing | Traditional Profilers | APM Solutions |
|---------|---------|---------------------|---------------|
| **AI/ML Focus** | ✅ Native PyTorch support | ❌ Generic profiling | ⚠️ Limited ML features |
| **Zero Setup** | ✅ Dynamic injection | ❌ Code instrumentation | ❌ Agent installation |
| **SQL Interface** | ✅ Full SQL support | ❌ Limited querying | ⚠️ Vendor-specific |
| **Real-time** | ✅ Live monitoring | ⚠️ Post-mortem analysis | ✅ Real-time |
| **Distributed** | ✅ Multi-node support | ❌ Single process | ✅ Distributed |
| **Cost** | ✅ Open source | ⚠️ Mixed | ❌ Expensive |

## Getting Started

Ready to dive deeper? Here's your recommended learning path:

### 🚀 **Start Here** (Essential - 30 minutes)
1. **[Installation](installation.md)** - Set up Probing on your system  
2. **[Quick Start](quick-start.md)** - Master `eval`, `query`, and `backtrace` with real examples
3. **[Basic Usage](../user-guide/basic-usage.md)** - Essential commands and workflows

### 🎯 **Deep Dive** (Choose based on your needs)
- **[SQL Analytics](../user-guide/sql-analytics.md)** - Advanced `query` techniques and data analysis
- **[Python Integration](../user-guide/python-integration.md)** - Power-user `eval` techniques  
- **[Debugging Guide](../user-guide/debugging.md)** - Expert `backtrace` usage for complex issues

### 🔧 **Production & Advanced** (When you're ready to scale)
- **[Production Deployment](../deployment/production.md)** - Scale Probing for production workloads
- **[Performance Tuning](../user-guide/performance.md)** - Optimize Probing itself for minimal overhead

### 💡 **Learn from Examples**
Browse `examples/` directory for real-world patterns:
- **Training debugging workflows** - Common PyTorch issues and solutions
- **Production monitoring setups** - Real deployment patterns  
- **Custom analytics queries** - Advanced SQL patterns for ML workloads

## Community and Support

- **GitHub Repository**: [github.com/reiase/probing](https://github.com/reiase/probing)
- **Documentation**: Browse this guide for comprehensive information
- **Examples**: Check the `examples/` directory for real-world usage patterns
- **Issues**: Report bugs and feature requests on GitHub

Probing is actively developed and welcomes community contributions. Whether you're fixing bugs, adding features, or improving documentation, your help makes Probing better for everyone.

---

**Next**: Learn how to [install Probing](installation.md) on your system.
