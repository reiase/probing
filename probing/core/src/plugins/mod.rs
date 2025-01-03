use std::sync::Mutex;

use anyhow::Result;
use once_cell::sync::Lazy;
use probing_engine::core::{
    ArrayRef, CustomSchema, DataType, Field, Float64Array, Int64Array, RecordBatch, Schema,
    SchemaPlugin, SchemaRef, StringArray,
};

use linux_taskstats;
use probing_proto::types::{TimeSeries, Value};

pub static TASK_STATS: Lazy<Mutex<TimeSeries>> = Lazy::new(|| {
    Mutex::new(
        TimeSeries::builder()
            .with_column(vec!["cpu_utime".to_string(), "cpu_stime".to_string()])
            .build(),
    )
});

pub fn task_stats_worker(iters: i64) -> Result<()> {
    let client = linux_taskstats::Client::open().map_err(|err| {
        println!("failed to open taskstats client: {}", err);
        err
    }).ok();
    let stats = client.unwrap().pid_stats(std::process::id()).map_err(|err| {
        println!("failed to get pid stats: {}", err);
        err
    })?;

    let mut iters = iters;
    while iters > 0 {
        let _ = TASK_STATS.lock().map(|mut ts| {
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as i64;
            let cpu_utime: Value = stats.cpu.utime_total.as_secs_f64().into();
            let cpu_stime: Value = stats.cpu.stime_total.as_secs_f64().into();
            let _ = ts.append(t.into(), vec![cpu_utime, cpu_stime]);
        });
        std::thread::sleep(std::time::Duration::from_millis(1));
        if iters > 0 {
            iters -= 1;
        }
    }
    Ok(())
}

#[derive(Default, Debug)]
pub struct TaskStatsSchema {}

impl CustomSchema for TaskStatsSchema {
    fn name() -> &'static str {
        "process"
    }

    fn list() -> Vec<String> {
        vec!["cpu".to_string(), "memory".to_string(), "io".to_string()]
    }

    fn data(expr: &str) -> Vec<RecordBatch> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_task_stats_worker() {
        super::task_stats_worker(1000).unwrap();
        super::TASK_STATS
            .lock()
            .map(|ts| {
                assert_eq!(ts.len(), 1000);
            })
            .unwrap();
    }
}
