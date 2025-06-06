# SQL Analytics Interface

Probing provides a powerful SQL interface for analyzing performance and monitoring data. This allows you to use familiar SQL syntax to query real-time and historical data from your applications.

## Overview

The SQL analytics interface transforms complex performance analysis into intuitive database queries. All monitoring data is accessible through standard SQL operations including `SELECT`, `WHERE`, `GROUP BY`, `ORDER BY`, and advanced analytical functions.

## Basic Query Structure

```sql
probing <pid> query "SELECT columns FROM table WHERE conditions"
```

## Core Tables

### System Information

**`system_info`** - Basic system and process information
```sql
SELECT * FROM system_info;
```

Common columns:
- `hostname` - Machine hostname
- `pid` - Process ID
- `cpu_usage` - Current CPU utilization
- `memory_usage` - Current memory usage
- `timestamp` - Data collection time

### Memory Analysis

**`memory_usage`** - Memory consumption over time
```sql
SELECT timestamp, used_memory_mb, available_memory_mb 
FROM memory_usage 
WHERE timestamp > now() - interval '1 hour';
```

### Performance Data

**`call_stats`** - Function call statistics
```sql
SELECT function_name, count(*) as calls, avg(duration_ms)
FROM call_stats 
GROUP BY function_name
ORDER BY calls DESC;
```

**`profiling_data`** - Detailed performance metrics
```sql
SELECT * FROM profiling_data 
WHERE duration_ms > 100
ORDER BY timestamp DESC;
```

## PyTorch Integration

When monitoring PyTorch applications, additional tables become available:

**`torch_training_logs`** - Training metrics
```sql
SELECT epoch, step, loss, accuracy, learning_rate
FROM torch_training_logs
WHERE epoch >= 5
ORDER BY step DESC;
```

**`torch_tensors`** - Tensor information
```sql
SELECT tensor_name, shape, dtype, device, memory_usage_mb
FROM torch_tensors
WHERE memory_usage_mb > 100;
```

## Advanced Analytics

### Time-Series Analysis

**Memory growth over time:**
```sql
SELECT 
  timestamp,
  used_memory_mb,
  used_memory_mb - LAG(used_memory_mb) OVER (ORDER BY timestamp) as memory_delta
FROM memory_usage
WHERE timestamp > now() - interval '30 minutes'
ORDER BY timestamp;
```

**Rolling averages:**
```sql
SELECT 
  timestamp,
  cpu_usage,
  AVG(cpu_usage) OVER (
    ORDER BY timestamp 
    ROWS BETWEEN 4 PRECEDING AND CURRENT ROW
  ) as cpu_avg_5min
FROM system_info
WHERE timestamp > now() - interval '1 hour';
```

### Performance Analysis

**Top slowest functions:**
```sql
SELECT 
  function_name,
  count(*) as call_count,
  avg(duration_ms) as avg_duration,
  max(duration_ms) as max_duration,
  stddev(duration_ms) as duration_stddev
FROM profiling_data
WHERE timestamp > now() - interval '10 minutes'
GROUP BY function_name
HAVING count(*) > 10
ORDER BY avg_duration DESC
LIMIT 10;
```

**Function call patterns:**
```sql
SELECT 
  DATE_TRUNC('minute', timestamp) as minute,
  function_name,
  count(*) as calls_per_minute
FROM call_stats
WHERE timestamp > now() - interval '1 hour'
GROUP BY minute, function_name
ORDER BY minute DESC, calls_per_minute DESC;
```

### Training Progress Analysis

**Loss convergence tracking:**
```sql
SELECT 
  epoch,
  step,
  loss,
  AVG(loss) OVER (
    PARTITION BY epoch 
    ORDER BY step
  ) as avg_loss_in_epoch
FROM torch_training_logs
WHERE epoch BETWEEN 1 AND 10
ORDER BY epoch, step;
```

**Learning rate scheduling:**
```sql
SELECT 
  epoch,
  min(learning_rate) as min_lr,
  max(learning_rate) as max_lr,
  avg(learning_rate) as avg_lr
FROM torch_training_logs
GROUP BY epoch
ORDER BY epoch;
```

## Data Filtering and Conditions

### Time-based Filtering

```sql
-- Last hour
WHERE timestamp > now() - interval '1 hour'

-- Last 24 hours
WHERE timestamp > now() - interval '1 day'

-- Specific time range
WHERE timestamp BETWEEN '2025-06-06 10:00:00' AND '2025-06-06 12:00:00'

-- Recent data only
WHERE timestamp > (SELECT max(timestamp) - interval '5 minutes' FROM table_name)
```

### Performance Filtering

```sql
-- High CPU usage periods
WHERE cpu_usage > 80

-- Memory growth detection
WHERE used_memory_mb > (
  SELECT avg(used_memory_mb) * 1.5 
  FROM memory_usage 
  WHERE timestamp > now() - interval '1 hour'
)

-- Slow function calls
WHERE duration_ms > 1000
```

## Aggregation Functions

### Statistical Functions

```sql
SELECT 
  function_name,
  count(*) as total_calls,
  avg(duration_ms) as mean_duration,
  percentile_cont(0.5) WITHIN GROUP (ORDER BY duration_ms) as median_duration,
  percentile_cont(0.95) WITHIN GROUP (ORDER BY duration_ms) as p95_duration,
  min(duration_ms) as min_duration,
  max(duration_ms) as max_duration,
  stddev(duration_ms) as std_duration
FROM profiling_data
GROUP BY function_name;
```

### Window Functions

```sql
SELECT 
  timestamp,
  memory_usage_mb,
  LAG(memory_usage_mb) OVER (ORDER BY timestamp) as prev_memory,
  LEAD(memory_usage_mb) OVER (ORDER BY timestamp) as next_memory,
  ROW_NUMBER() OVER (ORDER BY memory_usage_mb DESC) as memory_rank
FROM memory_usage
WHERE timestamp > now() - interval '1 hour';
```

## Cross-Table Joins

**Correlate system metrics with function performance:**
```sql
SELECT 
  s.timestamp,
  s.cpu_usage,
  s.memory_usage,
  p.function_name,
  p.duration_ms
FROM system_info s
JOIN profiling_data p ON abs(s.timestamp - p.timestamp) < interval '1 second'
WHERE s.timestamp > now() - interval '30 minutes'
  AND s.cpu_usage > 70;
```

## Configuration Tables

### View Current Settings

```sql
SELECT * FROM information_schema.df_settings 
WHERE name LIKE 'probing.%';
```

### System Configuration

```sql
SELECT * FROM information_schema.df_settings 
WHERE name LIKE 'server.%' OR name LIKE 'torch.%';
```

## Real-time Monitoring Queries

### Dashboard Queries

**System overview:**
```sql
SELECT 
  'CPU Usage' as metric,
  cpu_usage as value,
  '%' as unit
FROM system_info
WHERE timestamp = (SELECT max(timestamp) FROM system_info)
UNION ALL
SELECT 
  'Memory Usage',
  used_memory_mb,
  'MB'
FROM memory_usage
WHERE timestamp = (SELECT max(timestamp) FROM memory_usage);
```

**Training progress:**
```sql
SELECT 
  epoch,
  max(step) as current_step,
  min(loss) as best_loss,
  max(accuracy) as best_accuracy
FROM torch_training_logs
WHERE timestamp > now() - interval '1 hour'
GROUP BY epoch
ORDER BY epoch DESC
LIMIT 5;
```

## Export and Integration

### Data Export

Results can be exported for further analysis:

```bash
# Export to JSON
probing <pid> query "SELECT * FROM memory_usage" > memory_data.json

# Time-series data for plotting
probing <pid> query "
  SELECT timestamp, cpu_usage, memory_usage 
  FROM system_info 
  WHERE timestamp > now() - interval '1 hour'
" > system_metrics.json
```

### Integration with Other Tools

The SQL interface makes it easy to integrate with monitoring and visualization tools:

- Export data for Grafana dashboards
- Feed metrics into alerting systems
- Generate reports for analysis notebooks

## Best Practices

1. **Use time-based filtering** - Always include time constraints for better performance
2. **Limit result sets** - Use `LIMIT` clauses for large datasets
3. **Index-friendly queries** - Leverage timestamp and function_name columns
4. **Aggregate appropriately** - Use `GROUP BY` for summary statistics
5. **Test queries incrementally** - Start simple and add complexity gradually

## Common Query Patterns

### Performance Regression Detection

```sql
SELECT 
  function_name,
  avg(duration_ms) as current_avg,
  LAG(avg(duration_ms)) OVER (ORDER BY DATE_TRUNC('hour', timestamp)) as prev_avg
FROM profiling_data
WHERE timestamp > now() - interval '2 hours'
GROUP BY function_name, DATE_TRUNC('hour', timestamp)
HAVING avg(duration_ms) > LAG(avg(duration_ms)) OVER (ORDER BY DATE_TRUNC('hour', timestamp)) * 1.2;
```

### Memory Leak Detection

```sql
SELECT 
  DATE_TRUNC('minute', timestamp) as minute,
  max(used_memory_mb) - min(used_memory_mb) as memory_growth_mb
FROM memory_usage
WHERE timestamp > now() - interval '1 hour'
GROUP BY minute
HAVING max(used_memory_mb) - min(used_memory_mb) > 50
ORDER BY minute;
```

### Error Rate Analysis

```sql
SELECT 
  DATE_TRUNC('hour', timestamp) as hour,
  count(*) FILTER (WHERE error_count > 0) * 100.0 / count(*) as error_rate_percent
FROM call_stats
WHERE timestamp > now() - interval '24 hours'
GROUP BY hour
ORDER BY hour;
```

For more advanced usage patterns, see [Basic Usage](basic-usage.md) and [Memory Analysis](memory-analysis.md).
