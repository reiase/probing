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
    #[option(name = "torch.profiling_mode", aliases=["torch_profiling_mode"])]
    profiling_mode: Maybe<String>,
}

impl EngineCall for TorchExtension {}

impl EngineDatasource for TorchExtension {}

impl TorchExtension {
    fn set_profiling_mode(&mut self, profiling_mode: Maybe<String>) -> Result<(), EngineError> {
        match profiling_mode {
            Maybe::Nothing => Err(EngineError::InvalidOptionValue(
                "torch.profiling_mode".to_string(),
                profiling_mode.clone().into(),
            )),
            Maybe::Just(ref mode) => {
                match execute_python_code(&format!("probing.profiling.torch_probe.set_sampling_mode('{}')", mode)) {
                    Ok(_) => {
                        self.profiling_mode = profiling_mode.clone();
                        Ok(())
                    }
                    Err(_) => Err(EngineError::InvalidOptionValue(
                        "torch.profiling_mode".to_string(),
                        profiling_mode.clone().into(),
                    )),
                }
            }
        }
    }
}
