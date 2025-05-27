use std::time::Duration;

use datasrc::TaskStatsPlugin;
use probing_core::core::EngineCall;
use probing_core::core::EngineDatasource;
use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;

mod datasrc;

#[derive(Debug, Default, EngineExtension)]
pub struct TaskStatsExtension {
    /// Task statistics collection interval in milliseconds (0 to disable)
    #[option(aliases=["taskstats_interval"])]
    task_stats_interval: Maybe<i64>,
}

impl EngineCall for TaskStatsExtension {}

#[allow(unused)]
impl EngineDatasource for TaskStatsExtension {
    fn datasrc(
        &self,
        namespace: &str,
        name: Option<&str>,
    ) -> Option<std::sync::Arc<dyn probing_core::core::Plugin + Sync + Send>> {
        Some(TaskStatsPlugin::create(namespace))
    }
}

impl TaskStatsExtension {
    fn set_task_stats_interval(
        &mut self,
        task_stats_interval: Maybe<i64>,
    ) -> Result<(), EngineError> {
        match self.task_stats_interval {
            Maybe::Just(_) => Err(EngineError::InvalidOptionValue(
                Self::OPTION_TASK_STATS_INTERVAL.to_string(),
                task_stats_interval.clone().into(),
            )),
            Maybe::Nothing => match task_stats_interval {
                Maybe::Nothing => Err(EngineError::InvalidOptionValue(
                    Self::OPTION_TASK_STATS_INTERVAL.to_string(),
                    task_stats_interval.clone().into(),
                )),
                Maybe::Just(interval) => {
                    if interval < 0 {
                        return Err(EngineError::InvalidOptionValue(
                            Self::OPTION_TASK_STATS_INTERVAL.to_string(),
                            task_stats_interval.clone().into(),
                        ));
                    }
                    self.task_stats_interval = task_stats_interval.clone();
                    match datasrc::TaskStatsWorker::instance().start(datasrc::TaskStatsConfig {
                        interval: Duration::from_millis(interval as u64),
                        iterations: None,
                    }) {
                        Ok(_) => Ok(()),
                        Err(_) => Err(EngineError::InvalidOptionValue(
                            Self::OPTION_TASK_STATS_INTERVAL.to_string(),
                            interval.to_string(),
                        )),
                    }
                }
            },
        }
    }
}
