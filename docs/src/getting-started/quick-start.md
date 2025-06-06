# Quick Start

Get started with Probing in just a few minutes! This guide will walk you through injecting a probe into a Python process and performing your first analysis.

## Prerequisites

- Probing installed (see [Installation](installation.md))
- A running Python process or script to monitor

## Basic Workflow

### 1. Start Your Python Application

First, run any Python script or application:

```bash
# Example: Run a simple Python script
python -c "
import time
for i in range(1000):
    print(f'Processing {i}')
    time.sleep(1)
"
```

Or enable probing directly with environment variables:

```bash
# Enable probing from the start
PROBE=1 python your_script.py
```

### 2. Find the Process

List all Python processes to find your target:

```bash
probing list
```

This shows processes with injected probes. If your process isn't listed, note its PID from `ps` or `top`.

### 3. Inject the Probe

Inject probing capabilities into your running process:

```bash
# Replace <pid> with your process ID
probing <pid> inject
```

Example output:
```
Injecting /usr/local/bin/libprobing.so into 12345
Probe injection successful
```

### 4. View Real-time Information

Check the current stack trace:

```bash
probing <pid> backtrace
```

Query basic system information:

```bash
probing <pid> query "SELECT * FROM system_info"
```

### 5. Run Python Code Remotely

Execute Python code in the target process:

```bash
probing <pid> eval "import os; print(f'Working directory: {os.getcwd()}')"
```

## Quick Examples

### Memory Usage Analysis

```bash
# Check current memory usage
probing <pid> query "SELECT * FROM memory_usage"

# Monitor memory over time
probing <pid> query "
  SELECT timestamp, used_memory_mb, available_memory_mb 
  FROM memory_usage 
  WHERE timestamp > now() - interval '5 minutes'
"
```

### Performance Monitoring

```bash
# View function call statistics
probing <pid> query "SELECT * FROM call_stats LIMIT 10"

# Check CPU usage patterns
probing <pid> query "
  SELECT function_name, avg(duration_ms), count(*) as calls
  FROM profiling_data 
  GROUP BY function_name 
  ORDER BY avg(duration_ms) DESC
"
```

### PyTorch Integration

For PyTorch training processes:

```bash
# Monitor training variables
PROBE_TORCH_EXPRS="loss@train,accuracy@train" PROBE=1 python train.py

# Query training progress
probing <pid> query "
  SELECT epoch, step, loss, accuracy 
  FROM torch_training_logs 
  ORDER BY timestamp DESC 
  LIMIT 20
"
```

## Environment Variable Configuration

The `PROBE` environment variable provides quick setup:

| Value | Behavior |
|-------|----------|
| `1` or `followed` | Enable for current process only |
| `2` or `nested` | Enable for current and all child processes |
| `script:init.py+1` | Run initialization script and enable |

## Next Steps

- Learn more in [Basic Usage](../user-guide/basic-usage.md)
- Configure advanced features in [Troubleshooting](../user-guide/troubleshooting.md)
- Explore SQL analytics capabilities
- Check out distributed monitoring features

## Common First-Time Issues

If injection fails:
1. Ensure you have sufficient permissions (may need `sudo`)
2. Check that the target process is a Python process
3. Verify Probing is properly installed
4. See [Troubleshooting](../user-guide/troubleshooting.md) for detailed solutions
