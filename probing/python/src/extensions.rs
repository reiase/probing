use std::{ffi::CStr, time::Duration};

use probing_engine::core::{EngineError, EngineExtension, EngineExtensionOption};
use pyo3::Python;

use crate::{
    catch_crash::{enable_crash_handler, CRASH_HANDLER},
    get_code, pprof::PPROF_HOLDER,
};

#[derive(Debug, Default)]
pub struct PprofExtension {
    pprof_sample_freq: i32,
}

impl EngineExtension for PprofExtension {
    fn name(&self) -> String {
        "pprof".to_string()
    }

    fn set(&mut self, key: &str, value: &str) -> Result<String, EngineError> {
        match key {
            "pprof_sample_freq" | "pprof.sample_freq" | "pprof.sample.freq" => {
                let freq = value.parse::<i32>().map_err(|_| {
                    EngineError::InvalidOption(key.to_string(), value.to_string())
                })?;
                let old_value = format!("{}", self.pprof_sample_freq);
                self.pprof_sample_freq = freq;
                PPROF_HOLDER.setup(self.pprof_sample_freq);
                Ok(old_value)
            }
            _ => Err(EngineError::UnsupportedOption(key.to_string())),
        }
    }

    fn get(&self, key: &str) -> Result<String, EngineError> {
        match key {
            "pprof_sample_freq" | "pprof.sample_freq" | "pprof.sample.freq" => {
                Ok(self.pprof_sample_freq.to_string())
            }
            _ => Err(EngineError::UnsupportedOption(key.to_string())),
        }
    }

    fn options(&self) -> Vec<EngineExtensionOption> {
        vec![EngineExtensionOption {
            key: "pprof.sample_freq".to_string(),
            value: Some(self.pprof_sample_freq.to_string()),
            help: "CPU profiling sample frequency in Hz (higher values increase overhead)",
        }]
    }
}

#[derive(Debug, Default)]
pub struct TaskStatsExtension {
    task_stats_interval: i64,
}

impl EngineExtension for TaskStatsExtension {
    fn name(&self) -> String {
        "task_stats".to_string()
    }

    fn set(&mut self, key: &str, value: &str) -> Result<String, EngineError> {
        match key {
            "task_stats_interval" | "task_stats.interval" | "task.stats.interval" => {
                let old_value = format!("{}", self.task_stats_interval);
                let interval: i64 = value.parse().unwrap_or(0);
                self.task_stats_interval = interval;
                match probing_cc::TaskStatsWorker::instance().start(probing_cc::WorkerConfig {
                    interval: Duration::from_millis(interval as u64),
                    iterations: None,
                }) {
                    Ok(_) => Ok(old_value),
                    Err(e) => Err(EngineError::InvalidOption(
                        key.to_string(),
                        value.to_string(),
                    )),
                }
            }
            _ => Err(EngineError::UnsupportedOption(key.to_string())),
        }
    }

    fn get(&self, key: &str) -> Result<String, EngineError> {
        match key {
            "task_stats_interval" | "task_stats.interval" | "task.stats.interval" => {
                Ok(self.task_stats_interval.to_string())
            }
            _ => Err(EngineError::UnsupportedOption(key.to_string())),
        }
    }

    fn options(&self) -> Vec<EngineExtensionOption> {
        vec![EngineExtensionOption {
            key: "task_stats.interval".to_string(),
            value: Some(self.task_stats_interval.to_string()),
            help: "Task statistics collection interval in milliseconds (0 to disable)",
        }]
    }
}

#[derive(Debug, Default)]
pub struct TorchExtension {
    torch_sample_ratio: f64,
}

impl EngineExtension for TorchExtension {
    fn name(&self) -> String {
        "torch".to_string()
    }

    fn set(&mut self, key: &str, value: &str) -> Result<String, EngineError> {
        match key {
            "torch_sample_ratio" | "torch.sample_ratio" | "torch.sample.ratio" => {
                let old_value = format!("{}", self.torch_sample_ratio);
                let sample_ratio: f64 = value.parse().unwrap_or(0.0);
                let filename = format!("{}.py", "torch_profiling");
                let code = get_code(filename.as_str());
                match if let Some(code) = code {
                    Python::with_gil(|py| {
                        let code = format!("{}\0", code);
                        let code = CStr::from_bytes_with_nul(code.as_bytes())?;
                        py.run(code, None, None).map_err(|err| {
                            anyhow::anyhow!("error apply setting {}={}: {}", key, value, err)
                        })?;

                        let code = format!("torch_profiling({})\0", sample_ratio);
                        let code = CStr::from_bytes_with_nul(code.as_bytes())?;
                        py.run(code, None, None).map_err(|err| {
                            anyhow::anyhow!("error apply setting {}={}: {}", key, value, err)
                        })
                    })
                } else {
                    Err(anyhow::anyhow!("unsupported setting {}={}", key, value))
                } {
                    Ok(_) => Ok(old_value),
                    Err(err) => Err(EngineError::InvalidOption(
                        key.to_string(),
                        value.to_string(),
                    )),
                }
            }
            _ => Err(EngineError::UnsupportedOption(key.to_string())),
        }
    }

    fn get(&self, key: &str) -> Result<String, EngineError> {
        match key {
            "torch_sample_ratio" | "torch.sample_ratio" | "torch.sample.ratio" => {
                Ok(self.torch_sample_ratio.to_string())
            }
            _ => Err(EngineError::UnsupportedOption(key.to_string())),
        }
    }

    fn options(&self) -> Vec<EngineExtensionOption> {
        vec![EngineExtensionOption {
            key: "torch.sample_ratio".to_string(),
            value: Some(self.torch_sample_ratio.to_string()),
            help: "PyTorch profiler sampling ratio (0.0-1.0, where 1.0 profiles all operations)",
        }]
    }
}

#[derive(Debug, Default)]
pub struct PythonExtension {
    crash_handler: Option<String>,
}

impl EngineExtension for PythonExtension {
    fn name(&self) -> String {
        "python".to_string()
    }

    fn set(&mut self, key: &str, value: &str) -> Result<String, EngineError> {
        match key {
            "python.crash_handler" | "python.crash.handler" => {
                let old_value = self.crash_handler.clone().unwrap_or_default();
                self.crash_handler = Some(value.to_string());
                CRASH_HANDLER.lock().unwrap().replace(value.to_string());
                match enable_crash_handler() {
                    Ok(_) => Ok(old_value),
                    Err(e) => Err(EngineError::InvalidOption(
                        key.to_string(),
                        value.to_string(),
                    )),
                }
            }
            _ => Err(EngineError::UnsupportedOption(key.to_string())),
        }
    }

    fn get(&self, key: &str) -> Result<String, EngineError> {
        match key {
            "python.crash_handler" | "python.crash.handler" => {
                Ok(self.crash_handler.clone().unwrap_or_default())
            }
            _ => Err(EngineError::UnsupportedOption(key.to_string())),
        }
    }

    fn options(&self) -> Vec<EngineExtensionOption> {
        vec![EngineExtensionOption {
            key: "python.crash_handler".to_string(),
            value: self.crash_handler.clone(),
            help: "Path to Python crash handler script (executed when interpreter crashes)",
        }]
    }
}
