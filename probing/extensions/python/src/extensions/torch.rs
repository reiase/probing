use std::ffi::CStr;

use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;
use pyo3::Python;

use crate::get_code;

#[derive(Debug, Default, EngineExtension)]
pub struct TorchExtension {
    /// PyTorch profiler sampling ratio (0.0-1.0, where 1.0 profiles all operations)
    #[option(name = "torch.sample_ratio", aliases=["torch_sample_ratio"])]
    torch_sample_ratio: Maybe<f64>,
}

impl TorchExtension {
    fn set_torch_sample_ratio(
        &mut self,
        torch_sample_ratio: Maybe<f64>,
    ) -> Result<(), EngineError> {
        match self.torch_sample_ratio {
            Maybe::Just(_) => Err(EngineError::InvalidOption(
                "torch.sample_ratio".to_string(),
                torch_sample_ratio.clone().into(),
            )),
            Maybe::Nothing => match torch_sample_ratio {
                Maybe::Nothing => Err(EngineError::InvalidOption(
                    "torch.sample_ratio".to_string(),
                    torch_sample_ratio.clone().into(),
                )),
                Maybe::Just(sample_ratio) => {
                    if sample_ratio < 0.0 {
                        return Err(EngineError::InvalidOption(
                            "torch.sample_ratio".to_string(),
                            torch_sample_ratio.clone().into(),
                        ));
                    }
                    self.torch_sample_ratio = torch_sample_ratio.clone();

                    let filename = format!("{}.py", "torch_profiling");
                    let code = get_code(filename.as_str());
                    let key = "torch.sample_ratio".to_string();
                    let value = sample_ratio.to_string();
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
                        Ok(_) => Ok(()),
                        Err(err) => Err(EngineError::InvalidOption(key, value)),
                    }
                }
            },
        }
    }
}
