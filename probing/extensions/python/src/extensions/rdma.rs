use probing_core::core::EngineCall;
use probing_core::core::EngineDatasource;
use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;

use std::collections::HashMap;
use async_trait::async_trait;


#[derive(Debug, Default, EngineExtension)]
pub struct RdmaExtension {
    #[option(aliases=["sample.rate"])]
    sample_rate: Maybe<f64>,

    #[option(aliases=["hca.name"])]
    hca_name: Maybe<String>,
}

#[async_trait]
impl EngineCall for RdmaExtension {
    async fn call(
        &self,
        path: &str,
        params: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<Vec<u8>, EngineError> {
        if path == "/rdmaextension" {
            // 处理 /rdma 路径的逻辑
            println!("Handling RDMA request with params: {:?}", params);
            return Ok("RDMA request handled successfully".as_bytes().to_vec());
        }
        Err(EngineError::UnsupportedCall)
    }
}


impl EngineDatasource for RdmaExtension {}

impl RdmaExtension {
    fn set_sample_rate(&mut self, sample_rate: Maybe<f64>) -> Result<(), EngineError> {
        if let Maybe::Just(rate) = sample_rate {
            if !(0.0..=1.0).contains(&rate) {
                return Err(EngineError::InvalidOptionValue(
                    Self::OPTION_SAMPLE_RATE.to_string(),
                    rate.to_string(),
                ));
            }
        }
        self.sample_rate = sample_rate;
        Ok(())
    }

    fn set_hca_name(&mut self, hca_name: Maybe<String>) -> Result<(), EngineError> {
        // TODO: Add validation for watch variables format
        self.hca_name = hca_name;
        Ok(())
    }
}
