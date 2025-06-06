# Memory Analysis

Probing provides comprehensive memory analysis capabilities for Python applications, helping you identify memory leaks, optimize memory usage, and understand memory allocation patterns through PyTorch integration and Python evaluation capabilities.

## Overview

Memory analysis in Probing covers:
- PyTorch GPU memory monitoring
- Tensor memory analysis  
- Python memory monitoring via `psutil` and system tools
- Memory allocation patterns via torch traces
- Cross-step memory tracking

## PyTorch Memory Monitoring

### Current GPU Memory Status

Check current GPU memory usage:
```bash
probing $ENDPOINT eval "
import torch
if torch.cuda.is_available():
    for i in range(torch.cuda.device_count()):
        print(f'GPU {i}:')
        print(f'  Allocated: {torch.cuda.memory_allocated(i)/1024**3:.2f} GB')
        print(f'  Reserved:  {torch.cuda.memory_reserved(i)/1024**3:.2f} GB')
else:
    print('CUDA not available')
"
```

### Memory Trends via Torch Traces

View memory allocation trends during training:
```bash
probing $ENDPOINT query "
  SELECT step, stage, avg(allocated) as avg_memory_mb, max(allocated) as peak_memory_mb
  FROM python.torch_trace 
  WHERE step > (SELECT max(step) - 10 FROM python.torch_trace)
  GROUP BY step, stage
  ORDER BY step, stage
"
```

### Memory Growth Detection

Identify memory growth patterns:
```bash
probing $ENDPOINT query "
  SELECT 
    step,
    stage,
    allocated,
    LAG(allocated) OVER (PARTITION BY stage ORDER BY step, seq) as prev_memory,
    allocated - LAG(allocated) OVER (PARTITION BY stage ORDER BY step, seq) as memory_delta
  FROM python.torch_trace
  WHERE step > (SELECT max(step) - 5 FROM python.torch_trace)
    AND allocated > 0
  HAVING abs(memory_delta) > 50
"
```

## Python Memory Analysis

### Object Count Tracking

Monitor Python object counts:
```bash
probing $ENDPOINT eval "
import gc
objects = gc.get_objects()
from collections import Counter
types = Counter(type(obj).__name__ for obj in objects)
for obj_type, count in types.most_common(10):
    print(f'{obj_type}: {count}')
"
```

### Memory-Heavy Objects

Find objects consuming the most memory:
```bash
probing $ENDPOINT eval "
import sys
import gc

def get_size(obj):
    try:
        return sys.getsizeof(obj)
    except:
        return 0

objects = gc.get_objects()
large_objects = [(get_size(obj), type(obj).__name__, id(obj)) for obj in objects]
large_objects.sort(reverse=True)

print('Top 10 largest objects:')
for size, obj_type, obj_id in large_objects[:10]:
    print(f'{size:>10} bytes: {obj_type} (id: {obj_id})')
"
```

### Reference Counting

Analyze object references:
```bash
probing $ENDPOINT eval "
import sys
import gc

# Find objects with high reference counts
objects = gc.get_objects()
high_refs = [(sys.getrefcount(obj), type(obj).__name__, id(obj)) for obj in objects]
high_refs.sort(reverse=True)

print('Objects with highest reference counts:')
for ref_count, obj_type, obj_id in high_refs[:10]:
    print(f'{ref_count:>3} refs: {obj_type} (id: {obj_id})')
"
```

## Memory Leak Detection

### GPU Memory Leak Detection via Torch Traces

Monitor for GPU memory leaks during training:
```bash
probing $ENDPOINT query "
  SELECT 
    step,
    max(allocated) as peak_memory_mb,
    min(allocated) as min_memory_mb,
    max(allocated) - min(allocated) as memory_range_mb
  FROM python.torch_trace
  WHERE step > (SELECT max(step) - 20 FROM python.torch_trace)
  GROUP BY step
  HAVING max(allocated) - min(allocated) > 100  -- Alert if memory varies by >100MB in a step
  ORDER BY step DESC
"
```

### Memory Growth Rate Analysis

Calculate memory growth rates across training steps:
```bash
probing $ENDPOINT query "
  WITH memory_deltas AS (
    SELECT 
      step,
      stage,
      allocated,
      LAG(allocated) OVER (PARTITION BY stage ORDER BY step) as prev_allocated
    FROM python.torch_trace
    WHERE step > (SELECT max(step) - 10 FROM python.torch_trace)
      AND allocated > 0
  )
  SELECT 
    step,
    stage,
    allocated - prev_allocated as memory_growth_mb
  FROM memory_deltas
  WHERE prev_allocated IS NOT NULL
    AND allocated - prev_allocated > 50  -- Only show significant growth
  ORDER BY memory_growth_mb DESC
"
```

### Python Memory Leak Detection

Monitor memory usage with basic Python tools:
```bash
probing $ENDPOINT eval "
import gc
import psutil
import os

# Force garbage collection
gc.collect()

# Get current process memory usage
process = psutil.Process(os.getpid())
memory_info = process.memory_info()
rss_mb = memory_info.rss / 1024 / 1024
vms_mb = memory_info.vms / 1024 / 1024

print(f'Process Memory Usage:')
print(f'  RSS Memory: {rss_mb:.1f} MB')
print(f'  VMS Memory: {vms_mb:.1f} MB')

# Count objects by type
objects = gc.get_objects()
print(f'  Total objects: {len(objects)}')

# Check for uncollectable objects
uncollectable = gc.garbage
if uncollectable:
    print(f'  Uncollectable objects: {len(uncollectable)}')
else:
    print('  No uncollectable objects found')
"
```

### Memory Growth Rate Analysis via Python

Monitor memory growth using basic Python memory monitoring:
```bash
probing $ENDPOINT eval "
import psutil
import time
import os

# Get initial memory usage
process = psutil.Process(os.getpid())
initial_memory = process.memory_info().rss / 1024 / 1024

print(f'Initial memory: {initial_memory:.1f} MB')

# Wait and check again (in practice, you'd check periodically)
time.sleep(1)  # Brief pause for demo
current_memory = process.memory_info().rss / 1024 / 1024

growth = current_memory - initial_memory
print(f'Current memory: {current_memory:.1f} MB')
print(f'Memory growth: {growth:.1f} MB')

if growth > 10:  # Alert if growth > 10MB
    print('WARNING: Significant memory growth detected')
elif growth > 0:
    print('Memory is growing slightly')
else:
    print('Memory usage is stable')
"
```

### Garbage Collection Analysis

Monitor garbage collection behavior:
```bash
probing $ENDPOINT eval "
import gc

# Get GC stats
stats = gc.get_stats()
print('Garbage Collection Statistics:')
for i, stat in enumerate(stats):
    print(f'Generation {i}: {stat}')

# Force collection and see what's collected
before = len(gc.get_objects())
collected = gc.collect()
after = len(gc.get_objects())

print(f'\\nObjects before GC: {before}')
print(f'Objects collected: {collected}')
print(f'Objects after GC: {after}')
print(f'Net reduction: {before - after}')
"
```

## PyTorch Memory Analysis

### GPU Memory Monitoring

For PyTorch applications with GPU usage:
```bash
probing $ENDPOINT eval "
import torch

if torch.cuda.is_available():
    for i in range(torch.cuda.device_count()):
        print(f'GPU {i}:')
        print(f'  Allocated: {torch.cuda.memory_allocated(i)/1024**3:.2f} GB')
        print(f'  Reserved:  {torch.cuda.memory_reserved(i)/1024**3:.2f} GB')
        print(f'  Max Allocated: {torch.cuda.max_memory_allocated(i)/1024**3:.2f} GB')
        print(f'  Max Reserved:  {torch.cuda.max_memory_reserved(i)/1024**3:.2f} GB')
        print()
else:
    print('CUDA not available')
"
```

### Tensor Memory Analysis

Find large tensors:
```bash
probing $ENDPOINT eval "
import torch
import gc

tensors = [obj for obj in gc.get_objects() if isinstance(obj, torch.Tensor)]
tensor_info = []

for tensor in tensors:
    try:
        size_bytes = tensor.nelement() * tensor.element_size()
        size_mb = size_bytes / (1024 * 1024)
        tensor_info.append((size_mb, tensor.shape, tensor.dtype, tensor.device))
    except:
        continue

tensor_info.sort(reverse=True)

print('Largest tensors:')
for size_mb, shape, dtype, device in tensor_info[:15]:
    print(f'{size_mb:>8.2f} MB: {shape} {dtype} on {device}')
"
```

### Memory Fragmentation Analysis

Check for memory fragmentation:
```bash
probing $ENDPOINT eval "
import torch

if torch.cuda.is_available():
    print('CUDA Memory Fragmentation Analysis:')
    for i in range(torch.cuda.device_count()):
        allocated = torch.cuda.memory_allocated(i)
        reserved = torch.cuda.memory_reserved(i)
        
        if reserved > 0:
            fragmentation = (reserved - allocated) / reserved * 100
            print(f'GPU {i}: {fragmentation:.1f}% fragmented')
            print(f'  Allocated: {allocated/1024**3:.2f} GB')
            print(f'  Reserved:  {reserved/1024**3:.2f} GB')
            print(f'  Wasted:    {(reserved-allocated)/1024**3:.2f} GB')
"
```

## Advanced Memory Analysis

### Memory Profiling Integration

Create basic memory profiles using system tools:
```bash
probing $ENDPOINT eval "
import psutil
import gc
import os

# Get current process memory usage
process = psutil.Process(os.getpid())
memory_info = process.memory_info()

print(f'Memory Profile:')
print(f'  RSS Memory: {memory_info.rss / 1024 / 1024:.1f} MB')
print(f'  VMS Memory: {memory_info.vms / 1024 / 1024:.1f} MB')

# Analyze object counts
objects = gc.get_objects()
from collections import Counter
type_counts = Counter(type(obj).__name__ for obj in objects)

print(f'\\nTop 10 object types:')
for obj_type, count in type_counts.most_common(10):
    print(f'  {obj_type}: {count}')

# Check garbage collection stats
print(f'\\nGarbage Collection:')
for i, stat in enumerate(gc.get_stats()):
    print(f'  Generation {i}: {stat}')
"
```

### Memory Allocation Patterns via Torch Traces

Analyze GPU memory allocation patterns during training:
```bash
probing $ENDPOINT query "
  SELECT 
    step / 10 * 10 as step_range,  -- Group by 10-step ranges
    stage,
    avg(allocated) as avg_memory_mb,
    max(allocated) as peak_memory_mb,
    min(allocated) as min_memory_mb,
    count(*) as trace_count
  FROM python.torch_trace
  WHERE step > (SELECT max(step) - 100 FROM python.torch_trace)
    AND allocated > 0
  GROUP BY step_range, stage
  ORDER BY step_range, stage
"
```

### Cross-Process Memory Analysis

Compare memory usage across multiple processes using Python eval:
```bash
# Create a script to check multiple processes
for pid in 1234 5678 9012; do
  echo "=== Process $pid ==="
  probing $pid eval "
import os
import psutil

try:
    process = psutil.Process()
    memory_info = process.memory_info()
    print(f'PID: {os.getpid()}')
    print(f'RSS Memory: {memory_info.rss / 1024 / 1024:.1f} MB')
    print(f'VMS Memory: {memory_info.vms / 1024 / 1024:.1f} MB')
    
    # If PyTorch is available, also show GPU memory
    try:
        import torch
        if torch.cuda.is_available():
            allocated = torch.cuda.memory_allocated() / 1024**3
            reserved = torch.cuda.memory_reserved() / 1024**3
            print(f'GPU Allocated: {allocated:.2f} GB')
            print(f'GPU Reserved: {reserved:.2f} GB')
    except ImportError:
        pass
except Exception as e:
    print(f'Error getting memory info: {e}')
  "
done
```

## Memory Optimization Strategies

### Identifying Memory Hotspots via System Monitoring

Find memory usage patterns using system monitoring:
```bash
probing $ENDPOINT eval "
import psutil
import gc
import sys
from collections import defaultdict

# Get current process info
process = psutil.Process()
memory_info = process.memory_info()

print(f'System Memory Analysis:')
print(f'  RSS Memory: {memory_info.rss / 1024 / 1024:.1f} MB')
print(f'  VMS Memory: {memory_info.vms / 1024 / 1024:.1f} MB')
print(f'  Memory Percent: {process.memory_percent():.1f}%')

# Analyze large objects
objects = gc.get_objects()
large_objects = []

for obj in objects:
    try:
        size = sys.getsizeof(obj)
        if size > 1024 * 1024:  # Objects larger than 1MB
            large_objects.append((size, type(obj).__name__))
    except:
        continue

large_objects.sort(reverse=True)

print(f'\\nLarge objects (>1MB):')
for size, obj_type in large_objects[:10]:
    print(f'  {size / 1024 / 1024:.1f} MB: {obj_type}')

# Check for memory trends
import time
time.sleep(0.1)  # Brief pause
new_memory = process.memory_info().rss / 1024 / 1024
growth = new_memory - (memory_info.rss / 1024 / 1024)
print(f'\\nMemory growth in 0.1s: {growth:.2f} MB')
"
```

### Memory Usage Patterns by Training Phase

Analyze GPU memory usage patterns by training step ranges:
```bash
probing $ENDPOINT query "
  SELECT 
    CASE 
      WHEN step % 100 < 25 THEN 'Early Phase'
      WHEN step % 100 < 50 THEN 'Mid Phase' 
      WHEN step % 100 < 75 THEN 'Late Phase'
      ELSE 'End Phase'
    END as training_phase,
    avg(allocated) as avg_memory_mb,
    max(allocated) as peak_memory_mb,
    count(*) as trace_count
  FROM python.torch_trace
  WHERE step > (SELECT max(step) - 500 FROM python.torch_trace)
    AND allocated > 0
  GROUP BY training_phase
  ORDER BY avg_memory_mb DESC
"
```

## Alerts and Monitoring

### Memory Threshold Alerts via Python Monitoring

Set up alerts for memory usage using Python:
```bash
probing $ENDPOINT eval "
import psutil
import torch

# Check system memory
process = psutil.Process()
memory_mb = process.memory_info().rss / 1024 / 1024

if memory_mb > 8000:  # Alert above 8GB
    print(f'ALERT: HIGH_MEMORY_USAGE - {memory_mb:.1f} MB')
else:
    print(f'Memory usage normal: {memory_mb:.1f} MB')

# Check GPU memory if available
if torch.cuda.is_available():
    gpu_allocated_gb = torch.cuda.memory_allocated() / 1024**3
    gpu_reserved_gb = torch.cuda.memory_reserved() / 1024**3
    
    if gpu_allocated_gb > 10:  # Alert above 10GB
        print(f'ALERT: HIGH_GPU_MEMORY - Allocated: {gpu_allocated_gb:.2f} GB')
    else:
        print(f'GPU memory normal: {gpu_allocated_gb:.2f} GB allocated')
"
```

### Memory Leak Alerts via Torch Trace Analysis

Detect potential GPU memory leaks by monitoring allocation trends:
```bash
probing $ENDPOINT query "
  WITH memory_trend AS (
    SELECT 
      step,
      allocated,
      AVG(allocated) OVER (
        ORDER BY step 
        ROWS BETWEEN 10 PRECEDING AND CURRENT ROW
      ) as moving_avg_allocated
    FROM python.torch_trace
    WHERE step > (SELECT max(step) - 50 FROM python.torch_trace)
      AND allocated > 0
  )
  SELECT 
    step,
    allocated,
    moving_avg_allocated,
    'POTENTIAL_MEMORY_LEAK' as alert_type
  FROM memory_trend
  WHERE allocated > moving_avg_allocated * 1.2  -- 20% above moving average
  ORDER BY step DESC
"
```

## Best Practices

1. **Regular Monitoring** - Set up continuous memory monitoring
2. **Baseline Establishment** - Know your application's normal memory patterns
3. **Gradual Analysis** - Start with high-level views, then drill down
4. **Consider Context** - Memory usage often correlates with workload
5. **Clean Up** - Regularly analyze and clean up unnecessary objects

## Integration with Development Workflow

### Pre-deployment Memory Checks

Before deploying, analyze memory patterns using available data:
```bash
# Check for GPU memory growth during recent training
probing $ENDPOINT query "
  SELECT 
    max(allocated) - min(allocated) as memory_growth_mb,
    max(allocated) as peak_memory_mb,
    count(DISTINCT step) as steps_analyzed
  FROM python.torch_trace
  WHERE step > (SELECT max(step) - 100 FROM python.torch_trace)
    AND allocated > 0
"

# Also check Python memory usage
probing $ENDPOINT eval "
import psutil

process = psutil.Process()
rss_mb = process.memory_info().rss / 1024 / 1024

print(f'Memory analysis complete:')
print(f'  RSS Memory: {rss_mb:.1f} MB')
print(f'  Memory Percent: {process.memory_percent():.1f}%')

# Basic memory health check
if rss_mb > 8000:  # > 8GB
    print('  Status: HIGH MEMORY USAGE')
elif rss_mb > 4000:  # > 4GB  
    print('  Status: MODERATE MEMORY USAGE')
else:
    print('  Status: NORMAL MEMORY USAGE')
"
```

### Performance Regression Detection

Compare memory usage between different training runs:
```bash
# Export current PyTorch memory profile
probing $ENDPOINT query "
  SELECT 
    module,
    stage,
    avg(allocated) as avg_memory_mb,
    max(allocated) as peak_memory_mb
  FROM python.torch_trace
  WHERE step > (SELECT max(step) - 100 FROM python.torch_trace)
  GROUP BY module, stage
" > current_memory_profile.json

# Compare with previous runs by analyzing torch trace patterns
probing $ENDPOINT query "
  SELECT 
    step % 100 as relative_step,
    avg(allocated) as avg_memory_mb,
    stddev(allocated) as memory_variance
  FROM python.torch_trace  
  WHERE step > (SELECT max(step) - 200 FROM python.torch_trace)
  GROUP BY relative_step
  ORDER BY relative_step
" > memory_pattern_analysis.json
```

For more detailed analysis techniques, see [SQL Analytics](sql-analytics.md) and [Basic Usage](basic-usage.md).
