use probing_core::core::EngineCall;
use probing_core::core::EngineDatasource;
use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;

use super::python::execute_python_code;

#[derive(Debug, Default, EngineExtension)]
pub struct TorchExtension {
    /// PyTorch profiler mode to be used, "ordered:1.0" by default.
    #[option()]
    profiling_mode: Maybe<String>,

    /// PyTorch profiling sample rate (range: 0.0-1.0)
    #[option(aliases=["sample.rate"])]
    sample_rate: Maybe<f64>,

    /// Variables to capture during PyTorch profiling
    /// Format: <variable name>@<function name> (comma separated)
    #[option(aliases=["watch.vars", "watch_variables"])]
    watch_vars: Maybe<String>,
}

impl EngineCall for TorchExtension {}

impl EngineDatasource for TorchExtension {}

impl TorchExtension {
    fn set_profiling_mode(&mut self, profiling_mode: Maybe<String>) -> Result<(), EngineError> {
        match profiling_mode {
            Maybe::Nothing => Err(EngineError::InvalidOptionValue(
                Self::OPTION_PROFILING_MODE.to_string(),
                profiling_mode.clone().into(),
            )),
            Maybe::Just(ref mode) => {
                match execute_python_code(&format!(
                    "probing.profiling.torch_probe.set_sampling_mode('{}')",
                    mode
                )) {
                    Ok(_) => {
                        self.profiling_mode = profiling_mode.clone();
                        Ok(())
                    }
                    Err(_) => Err(EngineError::InvalidOptionValue(
                        Self::OPTION_PROFILING_MODE.to_string(),
                        profiling_mode.clone().into(),
                    )),
                }
            }
        }
    }

    fn set_sample_rate(&mut self, sample_rate: Maybe<f64>) -> Result<(), EngineError> {
        if let Maybe::Just(rate) = sample_rate {
            if rate < 0.0 || rate > 1.0 {
                return Err(EngineError::InvalidOptionValue(
                    Self::OPTION_SAMPLE_RATE.to_string(),
                    rate.to_string(),
                ));
            }
        }
        self.sample_rate = sample_rate;
        Ok(())
    }

    fn set_watch_vars(&mut self, watch_vars: Maybe<String>) -> Result<(), EngineError> {
        // TODO: Add validation for watch variables format
        self.watch_vars = watch_vars;
        Ok(())
    }
}
