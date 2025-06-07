# SQL Analytics Interface

Probing provides a powerful SQL interface for analyzing performance and monitoring data. This allows you to use familiar SQL syntax to query real-time and historical data from your applications.

## Overview

The SQL analytics interface transforms complex performance analysis into intuitive database queries. All monitoring data is accessible through standard SQL operations including `SELECT`, `WHERE`, `GROUP BY`, `ORDER BY`, and advanced analytical functions.

## Basic Query Structure

```sql
probing $ENDPOINT query "SELECT columns FROM table WHERE conditions"
```

## Core Tables

### Configuration and Metadata

**`information_schema.df_settings`** - System configuration and settings
```sql
SELECT * FROM information_schema.df_settings 
WHERE name LIKE 'probing.%';
```

Common columns:
- `name` - Configuration parameter name
- `value` - Configuration parameter value

### Python Namespace Tables

**`python.backtrace`** - Stack trace information
```sql
SELECT * FROM python.backtrace LIMIT 10;
```

Common columns:
- `ip` - Instruction pointer (for native frames)
- `file` - Source file name
- `func` - Function name
- `lineno` - Line number
- `depth` - Stack depth
- `frame_type` - Frame type ('Python' or 'Native')

### Dynamic External Tables

External tables can be created dynamically through the Python API:
```sql
-- Example: Query a custom external table
SELECT * FROM python.my_custom_table;
```

## PyTorch Integration

When monitoring PyTorch applications with the `@table` decorator, additional tables become available:

**`python.torch_trace`** - PyTorch execution traces
```sql
SELECT step, module, stage, duration, allocated
FROM python.torch_trace
WHERE step >= 5
ORDER BY step DESC, seq;
```

Common columns:
- `step` - Training step number
- `seq` - Sequence number within step
- `module` - Module name
- `stage` - Execution stage (forward, backward, step)
- `allocated` - GPU memory allocated (MB)
- `max_allocated` - Peak GPU memory allocated (MB)
- `cached` - GPU memory cached (MB)
- `max_cached` - Peak GPU memory cached (MB)
- `time_offset` - Time offset
- `duration` - Execution duration (seconds)

**`python.variables`** - Variable tracking
```sql
SELECT step, func, name, value
FROM python.variables
WHERE step = (SELECT max(step) FROM python.variables);
```

Common columns:
- `step` - Training step number
- `func` - Function name
- `name` - Variable name  
- `value` - Variable value (string representation)

## Advanced Analytics

### Time-Series Analysis

**Memory growth over time (using torch_trace):**
```sql
SELECT 
  step,
  stage,
  avg(allocated) as avg_memory_mb,
  max(allocated) as peak_memory_mb
FROM python.torch_trace
WHERE step > (SELECT max(step) - 10 FROM python.torch_trace)
GROUP BY step, stage
ORDER BY step, stage;
```

**Rolling averages:**
```sql
SELECT 
  step,
  module,
  duration,
  AVG(duration) OVER (
    PARTITION BY module
    ORDER BY step, seq
    ROWS BETWEEN 4 PRECEDING AND CURRENT ROW
  ) as avg_duration_5_samples
FROM python.torch_trace
WHERE step > (SELECT max(step) - 5 FROM python.torch_trace);
```

### Performance Analysis

**Top slowest operations:**
```sql
SELECT 
  module,
  stage,
  count(*) as execution_count,
  avg(duration) as avg_duration,
  max(duration) as max_duration,
  stddev(duration) as duration_stddev
FROM python.torch_trace
WHERE step > (SELECT max(step) - 10 FROM python.torch_trace)
  AND duration > 0
GROUP BY module, stage
HAVING count(*) > 5
ORDER BY avg_duration DESC
LIMIT 10;
```

**Execution patterns:**
```sql
SELECT 
  step,
  stage,
  count(*) as operations_per_step
FROM python.torch_trace
WHERE step > (SELECT max(step) - 5 FROM python.torch_trace)
GROUP BY step, stage
ORDER BY step DESC, operations_per_step DESC;
```

### Training Progress Analysis

**Memory usage trends during training:**
```sql
SELECT 
  step,
  avg(allocated) as avg_memory_allocated,
  max(allocated) as peak_memory_allocated,
  min(allocated) as min_memory_allocated
FROM python.torch_trace
WHERE step IS NOT NULL
GROUP BY step
ORDER BY step;
```

**Module execution time analysis:**
```sql
SELECT 
  module,
  stage,
  avg(duration) as avg_duration,
  count(*) as execution_count
FROM python.torch_trace
WHERE module IS NOT NULL 
  AND duration > 0
GROUP BY module, stage
ORDER BY avg_duration DESC;
```

## Data Filtering and Conditions

### Time-based Filtering

```sql
-- Recent steps only
WHERE step > (SELECT max(step) - 10 FROM python.torch_trace)

-- Specific step range
WHERE step BETWEEN 5 AND 15

-- Latest data only
WHERE step = (SELECT max(step) FROM python.torch_trace)
```

### Performance Filtering

```sql
-- Long-running operations
WHERE duration > 0.1

-- Memory-intensive operations
WHERE allocated > 1000  -- MB

-- Specific execution stages
WHERE stage IN ('forward', 'backward')

-- Specific modules
WHERE module LIKE '%attention%'
```

## Aggregation Functions

### Statistical Functions

```sql
SELECT 
  module,
  stage,
  count(*) as total_executions,
  avg(duration) as mean_duration,
  percentile_cont(0.5) WITHIN GROUP (ORDER BY duration) as median_duration,
  percentile_cont(0.95) WITHIN GROUP (ORDER BY duration) as p95_duration,
  min(duration) as min_duration,
  max(duration) as max_duration,
  stddev(duration) as std_duration
FROM python.torch_trace
WHERE duration > 0
GROUP BY module, stage;
```

### Window Functions

```sql
SELECT 
  step,
  allocated,
  LAG(allocated) OVER (ORDER BY step, seq) as prev_memory,
  LEAD(allocated) OVER (ORDER BY step, seq) as next_memory,
  ROW_NUMBER() OVER (ORDER BY allocated DESC) as memory_rank
FROM python.torch_trace
WHERE step > (SELECT max(step) - 5 FROM python.torch_trace);
```

## Cross-Table Joins

**Correlate torch traces with variable tracking:**
```sql
SELECT 
  t.step,
  t.module,
  t.duration,
  v.name as variable_name,
  v.value as variable_value
FROM python.torch_trace t
JOIN python.variables v ON t.step = v.step
WHERE t.step > (SELECT max(step) - 3 FROM python.torch_trace)
  AND t.duration > 0.05;
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

**Current training status:**
```sql
SELECT 
  'Current Step' as metric,
  max(step) as value,
  '' as unit
FROM python.torch_trace
UNION ALL
SELECT 
  'Peak Memory Usage',
  max(allocated),
  'MB'
FROM python.torch_trace
WHERE step = (SELECT max(step) FROM python.torch_trace);
```

**Training progress:**
```sql
SELECT 
  step,
  count(*) as total_operations,
  avg(duration) as avg_duration,
  max(allocated) as peak_memory_mb
FROM python.torch_trace
WHERE step > (SELECT max(step) - 5 FROM python.torch_trace)
GROUP BY step
ORDER BY step DESC
LIMIT 5;
```

## Export and Integration

### Data Export

Results can be exported for further analysis:

```bash
# Export to JSON
probing $ENDPOINT query "SELECT * FROM python.torch_trace" > torch_traces.json

# Time-series data for plotting
probing $ENDPOINT query "
  SELECT step, stage, avg(duration), avg(allocated)
  FROM python.torch_trace 
  WHERE step > (SELECT max(step) - 10 FROM python.torch_trace)
  GROUP BY step, stage
" > training_metrics.json
```

### Integration with Other Tools

The SQL interface makes it easy to integrate with monitoring and visualization tools:

- Export data for Grafana dashboards
- Feed metrics into alerting systems
- Generate reports for analysis notebooks

## Best Practices

1. **Use step-based filtering** - Always include step constraints for better performance
2. **Limit result sets** - Use `LIMIT` clauses for large datasets
3. **Index-friendly queries** - Leverage step and module columns
4. **Aggregate appropriately** - Use `GROUP BY` for summary statistics
5. **Test queries incrementally** - Start simple and add complexity gradually

## Common Query Patterns

### Performance Regression Detection

```sql
SELECT 
  module,
  stage,
  avg(duration) as current_avg,
  LAG(avg(duration)) OVER (ORDER BY step) as prev_avg
FROM python.torch_trace
WHERE step > (SELECT max(step) - 10 FROM python.torch_trace)
GROUP BY module, stage, step
HAVING avg(duration) > LAG(avg(duration)) OVER (ORDER BY step) * 1.2;
```

### Memory Usage Growth Detection

```sql
SELECT 
  step,
  max(allocated) - min(allocated) as memory_growth_mb
FROM python.torch_trace
WHERE step > (SELECT max(step) - 5 FROM python.torch_trace)
GROUP BY step
HAVING max(allocated) - min(allocated) > 50
ORDER BY step;
```

### Error Rate Analysis

For custom external tables with error tracking:
```sql
SELECT 
  step,
  count(*) FILTER (WHERE error_count > 0) * 100.0 / count(*) as error_rate_percent
FROM python.my_error_table
WHERE step > (SELECT max(step) - 20 FROM python.my_error_table)
GROUP BY step
ORDER BY step;
```

For more advanced usage patterns, see [Basic Usage](basic-usage.md) and [Memory Analysis](memory-analysis.md).
