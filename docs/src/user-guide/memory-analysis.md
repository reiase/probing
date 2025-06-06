# Memory Analysis

Probing provides comprehensive memory analysis capabilities for Python applications, helping you identify memory leaks, optimize memory usage, and understand memory allocation patterns.

## Overview

Memory analysis in Probing covers:
- Real-time memory usage monitoring
- Memory leak detection
- Object lifecycle tracking
- PyTorch tensor memory analysis
- Memory allocation patterns

## Basic Memory Monitoring

### Current Memory Status

Check current memory usage:
```bash
probing <pid> query "SELECT * FROM memory_usage ORDER BY timestamp DESC LIMIT 1"
```

View memory trend over time:
```bash
probing <pid> query "
  SELECT timestamp, used_memory_mb, available_memory_mb
  FROM memory_usage 
  WHERE timestamp > now() - interval '1 hour'
  ORDER BY timestamp
"
```

### Memory Growth Detection

Identify memory growth patterns:
```bash
probing <pid> query "
  SELECT 
    timestamp,
    used_memory_mb,
    used_memory_mb - LAG(used_memory_mb) OVER (ORDER BY timestamp) as memory_delta
  FROM memory_usage
  WHERE timestamp > now() - interval '30 minutes'
  HAVING abs(memory_delta) > 10
"
```

## Python Object Analysis

### Object Count Tracking

Monitor Python object counts:
```bash
probing <pid> eval "
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
probing <pid> eval "
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
probing <pid> eval "
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

### Automatic Leak Detection

Set up continuous monitoring for potential leaks:
```bash
probing <pid> query "
  SELECT 
    DATE_TRUNC('minute', timestamp) as minute,
    max(used_memory_mb) as peak_memory,
    min(used_memory_mb) as min_memory,
    max(used_memory_mb) - min(used_memory_mb) as memory_range
  FROM memory_usage
  WHERE timestamp > now() - interval '2 hours'
  GROUP BY minute
  HAVING memory_range > 100  -- Alert if memory varies by >100MB in a minute
  ORDER BY minute DESC
"
```

### Memory Growth Rate Analysis

Calculate memory growth rates:
```bash
probing <pid> query "
  WITH memory_deltas AS (
    SELECT 
      timestamp,
      used_memory_mb,
      used_memory_mb - LAG(used_memory_mb) OVER (ORDER BY timestamp) as delta_mb,
      extract(epoch from (timestamp - LAG(timestamp) OVER (ORDER BY timestamp))) as delta_seconds
    FROM memory_usage
    WHERE timestamp > now() - interval '1 hour'
  )
  SELECT 
    timestamp,
    delta_mb,
    CASE 
      WHEN delta_seconds > 0 THEN delta_mb / delta_seconds * 60  -- MB per minute
      ELSE 0 
    END as growth_rate_mb_per_min
  FROM memory_deltas
  WHERE delta_mb > 5  -- Only show significant changes
  ORDER BY growth_rate_mb_per_min DESC
"
```

### Garbage Collection Analysis

Monitor garbage collection behavior:
```bash
probing <pid> eval "
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
probing <pid> eval "
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

Analyze tensor memory usage:
```bash
probing <pid> query "SELECT * FROM torch_tensors ORDER BY memory_usage_mb DESC LIMIT 20"
```

Find large tensors:
```bash
probing <pid> eval "
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
probing <pid> eval "
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

Create detailed memory profiles:
```bash
probing <pid> eval "
import tracemalloc
import linecache
import os

# Start tracing if not already started
if not tracemalloc.is_tracing():
    tracemalloc.start()

# Get current memory usage
current, peak = tracemalloc.get_traced_memory()
print(f'Current memory usage: {current / 1024 / 1024:.1f} MB')
print(f'Peak memory usage: {peak / 1024 / 1024:.1f} MB')

# Get top memory allocations
snapshot = tracemalloc.take_snapshot()
top_stats = snapshot.statistics('lineno')

print('\\nTop 10 memory allocations:')
for index, stat in enumerate(top_stats[:10], 1):
    frame = stat.traceback.format()[-1]
    print(f'{index:2d}. {stat.size / 1024 / 1024:.1f} MB - {frame}')
"
```

### Memory Allocation Patterns

Analyze allocation patterns over time:
```bash
probing <pid> query "
  SELECT 
    DATE_TRUNC('hour', timestamp) as hour,
    avg(used_memory_mb) as avg_memory,
    max(used_memory_mb) as peak_memory,
    min(used_memory_mb) as min_memory,
    stddev(used_memory_mb) as memory_volatility
  FROM memory_usage
  WHERE timestamp > now() - interval '24 hours'
  GROUP BY hour
  ORDER BY hour
"
```

### Cross-Process Memory Analysis

Compare memory usage across multiple processes:
```bash
# Run this for each process you want to compare
for pid in 1234 5678 9012; do
  echo "=== Process $pid ==="
  probing $pid query "
    SELECT 
      pid, 
      used_memory_mb, 
      cpu_usage,
      timestamp
    FROM system_info 
    ORDER BY timestamp DESC 
    LIMIT 1
  "
done
```

## Memory Optimization Strategies

### Identifying Memory Hotspots

Find functions that allocate the most memory:
```bash
probing <pid> query "
  SELECT 
    function_name,
    count(*) as call_count,
    avg(memory_delta_mb) as avg_memory_per_call,
    sum(memory_delta_mb) as total_memory_allocated
  FROM memory_allocations
  WHERE timestamp > now() - interval '1 hour'
  GROUP BY function_name
  ORDER BY total_memory_allocated DESC
  LIMIT 20
"
```

### Memory Usage Patterns

Analyze memory usage patterns by time of day:
```bash
probing <pid> query "
  SELECT 
    extract(hour from timestamp) as hour_of_day,
    avg(used_memory_mb) as avg_memory,
    max(used_memory_mb) as peak_memory
  FROM memory_usage
  WHERE timestamp > now() - interval '7 days'
  GROUP BY hour_of_day
  ORDER BY hour_of_day
"
```

## Alerts and Monitoring

### Memory Threshold Alerts

Set up alerts for memory usage:
```bash
probing <pid> query "
  SELECT 
    timestamp,
    used_memory_mb,
    'HIGH_MEMORY_USAGE' as alert_type
  FROM memory_usage
  WHERE used_memory_mb > 8000  -- Alert above 8GB
    AND timestamp > now() - interval '5 minutes'
"
```

### Memory Leak Alerts

Detect potential memory leaks:
```bash
probing <pid> query "
  WITH memory_trend AS (
    SELECT 
      timestamp,
      used_memory_mb,
      AVG(used_memory_mb) OVER (
        ORDER BY timestamp 
        ROWS BETWEEN 10 PRECEDING AND CURRENT ROW
      ) as moving_avg
    FROM memory_usage
    WHERE timestamp > now() - interval '1 hour'
  )
  SELECT *
  FROM memory_trend
  WHERE used_memory_mb > moving_avg * 1.2  -- 20% above moving average
    AND timestamp > now() - interval '10 minutes'
"
```

## Best Practices

1. **Regular Monitoring** - Set up continuous memory monitoring
2. **Baseline Establishment** - Know your application's normal memory patterns
3. **Gradual Analysis** - Start with high-level views, then drill down
4. **Consider Context** - Memory usage often correlates with workload
5. **Clean Up** - Regularly analyze and clean up unnecessary objects

## Integration with Development Workflow

### Pre-deployment Checks

Before deploying, run memory analysis:
```bash
# Check for memory leaks during testing
probing <pid> query "
  SELECT 
    max(used_memory_mb) - min(used_memory_mb) as memory_growth_mb
  FROM memory_usage
  WHERE timestamp > now() - interval '1 hour'
"
```

### Performance Regression Detection

Compare memory usage between versions:
```bash
# Export current memory profile
probing <pid> query "
  SELECT function_name, avg(memory_usage_mb)
  FROM memory_allocations
  GROUP BY function_name
" > current_memory_profile.json
```

For more detailed analysis techniques, see [SQL Analytics](sql-analytics.md) and [Basic Usage](basic-usage.md).
