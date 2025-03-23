use std::time::Duration;

use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;

#[derive(Debug, Default, EngineExtension)]
pub struct TaskStatsExtension {
    /// Task statistics collection interval in milliseconds (0 to disable)
    #[option(name = "taskstats.interval", aliases=["taskstats_interval"])]
    task_stats_interval: Maybe<i64>,
}

impl TaskStatsExtension {
    fn set_task_stats_interval(
        &mut self,
        task_stats_interval: Maybe<i64>,
    ) -> Result<(), EngineError> {
        match self.task_stats_interval {
            Maybe::Just(_) => Err(EngineError::InvalidOptionValue(
                "taskstats.interval".to_string(),
                task_stats_interval.clone().into(),
            )),
            Maybe::Nothing => match task_stats_interval {
                Maybe::Nothing => Err(EngineError::InvalidOptionValue(
                    "taskstats.interval".to_string(),
                    task_stats_interval.clone().into(),
                )),
                Maybe::Just(interval) => {
                    if interval < 0 {
                        return Err(EngineError::InvalidOptionValue(
                            "taskstats.interval".to_string(),
                            task_stats_interval.clone().into(),
                        ));
                    }
                    self.task_stats_interval = task_stats_interval.clone();
                    match probing_cc::TaskStatsWorker::instance().start(
                        probing_cc::TaskStatsConfig {
                            interval: Duration::from_millis(interval as u64),
                            iterations: None,
                        },
                    ) {
                        Ok(_) => Ok(()),
                        Err(e) => Err(EngineError::InvalidOptionValue(
                            "taskstats.interval".to_string(),
                            interval.to_string(),
                        )),
                    }
                }
            },
        }
    }
}
