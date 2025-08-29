use probing_core::core::EngineCall;
use probing_core::core::EngineDatasource;
use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;

#[derive(Debug, Default, EngineExtension)]
pub struct PprofExtension {
    /// CPU profiling sample frequency in Hz (higher values increase overhead)
    #[option(aliases=["sample.freq"])]
    sample_freq: Maybe<i32>,
}

impl EngineCall for PprofExtension {}

impl EngineDatasource for PprofExtension {}

impl PprofExtension {
    fn set_sample_freq(&mut self, pprof_sample_freq: Maybe<i32>) -> Result<(), EngineError> {
        match self.sample_freq {
            Maybe::Just(_) => Err(EngineError::InvalidOptionValue(
                Self::OPTION_SAMPLE_FREQ.to_string(),
                pprof_sample_freq.clone().into(),
            )),
            Maybe::Nothing => match pprof_sample_freq {
                Maybe::Nothing => Err(EngineError::InvalidOptionValue(
                    Self::OPTION_SAMPLE_FREQ.to_string(),
                    pprof_sample_freq.clone().into(),
                )),
                Maybe::Just(freq) => {
                    if freq < 1 {
                        return Err(EngineError::InvalidOptionValue(
                            Self::OPTION_SAMPLE_FREQ.to_string(),
                            pprof_sample_freq.clone().into(),
                        ));
                    }
                    self.sample_freq = pprof_sample_freq.clone();
                    crate::features::pprof::setup(freq as u64).map_err(|e| {
                        EngineError::InvalidOptionValue(
                            Self::OPTION_SAMPLE_FREQ.to_string(),
                            e.to_string(),
                        )
                    })?;
                    Ok(())
                }
            },
        }
    }
}
