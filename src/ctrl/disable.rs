use anyhow::Result;
use probing_ppp::cli::Features;

use crate::handlers::PPROF_HOLDER;

pub fn handle(feature: Features) -> Result<String> {
    match feature {
        Features::Pprof => {
            PPROF_HOLDER.reset();
            Ok("pprof is  disabled".to_string())
        }
        Features::Dap { address: _ } => Ok("unable to disable dap".to_string()),
        Features::Remote { address: _ } => Ok("unable to disable remote connection".to_string()),
        Features::CatchCrash { address: _ } => Ok("unable to disable crash handler".to_string()),
    }
}
