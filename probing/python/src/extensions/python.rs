use probing_engine::core::EngineError;
use probing_engine::core::EngineExtension;
use probing_engine::core::EngineExtensionOption;
use probing_engine::core::Maybe;

use crate::catch_crash::enable_crash_handler;
use crate::catch_crash::CRASH_HANDLER;

#[derive(Debug, Default, EngineExtension)]
pub struct PythonExtension {
    /// Path to Python crash handler script (executed when interpreter crashes)
    #[option(name="python.crash_handler", aliases=["python.crash.handler"])]
    crash_handler: Maybe<String>,
}

impl PythonExtension {
    fn set_crash_handler(&mut self, crash_handler: Maybe<String>) -> Result<(), EngineError> {
        match self.crash_handler {
            Maybe::Just(_) => Err(EngineError::InvalidOption(
                "python.crash_handler".to_string(),
                crash_handler.clone().into(),
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
}
