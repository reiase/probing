use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;

use crate::python::enable_crash_handler;
use crate::python::enable_monitoring;
use crate::python::CRASH_HANDLER;

#[derive(Debug, Default, EngineExtension)]
pub struct PythonExtension {
    /// Path to Python crash handler script (executed when interpreter crashes)
    #[option(name="python.crash_handler", aliases=["python.crash.handler"])]
    crash_handler: Maybe<String>,

    /// Path to Python Monitoring Handler
    #[option(name = "python.monitoring")]
    monitoring: Maybe<String>,
}

impl PythonExtension {
    fn set_crash_handler(&mut self, crash_handler: Maybe<String>) -> Result<(), EngineError> {
        match self.crash_handler {
            Maybe::Just(_) => Err(EngineError::ReadOnlyOption(
                "python.crash_handler".to_string(),
            )),
            Maybe::Nothing => match &crash_handler {
                Maybe::Nothing => Err(EngineError::InvalidOption(
                    "python.crash_handler".to_string(),
                    crash_handler.clone().into(),
                )),
                Maybe::Just(handler) => {
                    self.crash_handler = crash_handler.clone();
                    CRASH_HANDLER.lock().unwrap().replace(handler.to_string());
                    match enable_crash_handler() {
                        Ok(_) => Ok(()),
                        Err(e) => Err(EngineError::InvalidOption(
                            "python.crash_handler".to_string(),
                            handler.to_string(),
                        )),
                    }
                }
            },
        }
    }

    fn set_monitoring(&mut self, monitoring: Maybe<String>) -> Result<(), EngineError> {
        log::debug!("setting python.monitoring = {}", monitoring);
        match self.monitoring {
            Maybe::Just(_) => Err(EngineError::ReadOnlyOption("python.monitoring".to_string())),
            Maybe::Nothing => match &monitoring {
                Maybe::Nothing => Err(EngineError::InvalidOption(
                    "python.monitoring".to_string(),
                    monitoring.clone().into(),
                )),
                Maybe::Just(handler) => {
                    self.monitoring = monitoring.clone();
                    match enable_monitoring(handler) {
                        Ok(_) => Ok(()),
                        Err(e) => Err(EngineError::InvalidOption(
                            "python.monitoring".to_string(),
                            handler.to_string(),
                        )),
                    }
                }
            },
        }
    }
}
