use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;

use crate::pprof::PPROF_HOLDER;

#[derive(Debug, Default, EngineExtension)]
pub struct PprofExtension {
    /// CPU profiling sample frequency in Hz (higher values increase overhead)
    #[option(name = "pprof.sample_freq", aliases=["pprof_sample_freq", "pprof.sample.freq"])]
    pprof_sample_freq: Maybe<i32>,
}

impl PprofExtension {
    fn set_pprof_sample_freq(&mut self, pprof_sample_freq: Maybe<i32>) -> Result<(), EngineError> {
        match self.pprof_sample_freq {
            Maybe::Just(_) => Err(EngineError::InvalidOption(
                "pprof.sample_freq".to_string(),
                pprof_sample_freq.clone().into(),
            )),
            Maybe::Nothing => match pprof_sample_freq {
                Maybe::Nothing => Err(EngineError::InvalidOption(
                    "pprof.sample_freq".to_string(),
                    pprof_sample_freq.clone().into(),
                )),
                Maybe::Just(freq) => {
                    if freq < 1 {
                        return Err(EngineError::InvalidOption(
                            "pprof.sample_freq".to_string(),
                            pprof_sample_freq.clone().into(),
                        ));
                    }
                    self.pprof_sample_freq = pprof_sample_freq.clone();
                    PPROF_HOLDER.setup(freq);
                    Ok(())
                }
            },
        }
    }
}
