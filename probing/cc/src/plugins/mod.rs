use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::Result;
use once_cell::sync::Lazy;
use thiserror::Error;

use probing_engine::core::{CustomSchema, RecordBatch};

use probing_proto::types::{TimeSeries, Value};

#[derive(Error, Debug)]
pub enum WorkerError {
    #[error("Worker already running")]
    AlreadyRunning,
    #[error("Failed to start worker: {0}")]
    StartError(String),
    #[error("Failed to stop worker: {0}")]
    StopError(String),
    #[error("Taskstats error: {0}")]
    TaskStatsError(#[from] linux_taskstats::Error),
}

pub struct WorkerConfig {
    pub interval: Duration,
    pub iterations: Option<i64>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(1),
            iterations: None,
        }
    }
}

pub struct TaskStatsWorker {
    running: Arc<AtomicBool>,
    config: WorkerConfig,
    time_series: Arc<Mutex<TimeSeries>>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl TaskStatsWorker {
    pub fn instance() -> &'static Self {
        static INSTANCE: Lazy<TaskStatsWorker> = Lazy::new(|| TaskStatsWorker {
            running: Arc::new(AtomicBool::new(false)),
            config: WorkerConfig::default(),
            time_series: Arc::new(Mutex::new(
                TimeSeries::builder()
                    .with_columns(vec!["cpu_utime".to_string(), "cpu_stime".to_string()])
                    .build(),
            )),
            handle: Mutex::new(None),
        });
        &INSTANCE
    }

    pub fn start(&self, config: WorkerConfig) -> Result<(), WorkerError> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(WorkerError::AlreadyRunning);
        }

        let running = self.running.clone();
        let time_series = self.time_series.clone();

        let handle = thread::spawn(move || {
            let client = match linux_taskstats::Client::open() {
                Ok(client) => client,
                Err(e) => {
                    log::error!("Failed to open taskstats client: {}", e);
                    return;
                }
            };

            let mut iterations = config.iterations;
            while running.load(Ordering::SeqCst) {
                if let Some(iter) = iterations.as_mut() {
                    if *iter <= 0 {
                        break;
                    }
                    *iter -= 1;
                }

                match client.pid_stats(std::process::id()) {
                    Ok(stats) => {
                        if let Ok(mut ts) = time_series.lock() {
                            let t = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_micros() as i64;
                            let cpu_utime: Value = stats.cpu.utime_total.as_secs_f64().into();
                            let cpu_stime: Value = stats.cpu.stime_total.as_secs_f64().into();
                            match ts.append(t.into(), vec![cpu_utime, cpu_stime]) {
                                Ok(_) => {}
                                Err(e) => log::error!("Failed to append to time series: {}", e),
                            };
                        }
                    }
                    Err(e) => log::error!("Failed to get pid stats: {}", e),
                }

                thread::sleep(config.interval);
            }
        });

        *self.handle.lock().unwrap() = Some(handle);
        Ok(())
    }

    pub fn stop(&self) -> Result<(), WorkerError> {
        if !self.running.swap(false, Ordering::SeqCst) {
            return Ok(());
        }

        if let Some(handle) = self.handle.lock().unwrap().take() {
            handle
                .join()
                .map_err(|_| WorkerError::StopError("Thread join failed".into()))?;
        }

        Ok(())
    }

    pub fn get_stats(&self) -> Result<TimeSeries, WorkerError> {
        Ok(self.time_series.lock().unwrap().clone())
    }
}

// pub static TASK_STATS: Lazy<Mutex<TimeSeries>> = Lazy::new(|| {
//     Mutex::new(
//         TimeSeries::builder()
//             .with_columns(vec!["cpu_utime".to_string(), "cpu_stime".to_string()])
//             .build(),
//     )
// });

// pub fn task_stats_worker(iters: i64) -> Result<()> {
//     let client = linux_taskstats::Client::open()
//         .map_err(|err| {
//             println!("failed to open taskstats client: {}", err);
//             err
//         })
//         .ok();
//     let stats = client
//         .unwrap()
//         .pid_stats(std::process::id())
//         .map_err(|err| {
//             println!("failed to get pid stats: {}", err);
//             err
//         })?;

//     let mut iters = iters;
//     while iters > 0 {
//         let _ = TASK_STATS.lock().map(|mut ts| {
//             let t = std::time::SystemTime::now()
//                 .duration_since(std::time::UNIX_EPOCH)
//                 .unwrap()
//                 .as_micros() as i64;
//             let cpu_utime: Value = stats.cpu.utime_total.as_secs_f64().into();
//             let cpu_stime: Value = stats.cpu.stime_total.as_secs_f64().into();
//             let _ = ts.append(t.into(), vec![cpu_utime, cpu_stime]);
//         });
//         std::thread::sleep(std::time::Duration::from_millis(1));
//         if iters > 0 {
//             iters -= 1;
//         }
//     }
//     Ok(())
// }

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
    use super::{TaskStatsWorker, WorkerConfig};

    #[test]
    fn test_task_stats_worker() {
        TaskStatsWorker::instance()
            .start(WorkerConfig {
                interval: std::time::Duration::from_millis(1),
                iterations: Some(1000),
            })
            .unwrap();

        std::thread::sleep(std::time::Duration::from_secs(2));
        let length = TaskStatsWorker::instance().get_stats().unwrap().len();
        
        assert_eq!(length, 1000);
    }
}
