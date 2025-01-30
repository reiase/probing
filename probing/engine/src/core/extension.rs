use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use datafusion::config::{ConfigExtension, ExtensionOptions};

use super::error::EngineError;

pub struct EngineExtensionOption {
    pub key: String,
    pub value: Option<String>,
    pub help: &'static str,
}

pub trait EngineExtension: Debug + Send + Sync {
    fn name(&self) -> String;
    fn set(&mut self, key: &str, value: &str) -> Result<String, EngineError>;
    fn get(&self, key: &str) -> Result<String, EngineError>;
    fn options(&self) -> Vec<EngineExtensionOption>;
}

#[derive(Debug)]
pub struct EngineExtensionManager {
    extensions: Vec<Arc<Mutex<dyn EngineExtension + Send + Sync>>>,
}

impl EngineExtensionManager {
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
        }
    }

    pub fn register(&mut self, extension: Arc<Mutex<dyn EngineExtension + Send + Sync>>) {
        self.extensions.push(extension);
    }

    pub fn set_option(&mut self, key: &str, value: &str) -> Result<(), EngineError> {
        for extension in &self.extensions {
            if let Ok(mut ext) = extension.lock() {
                match ext.set(key, value) {
                    Ok(old) => {
                        log::info!("setting update [{}]:{key}={value} <= {old}", ext.name());
                        return Ok(());
                    }
                    Err(EngineError::UnsupportedOption(_)) => continue,
                    Err(e) => return Err(e),
                }
            }
        }
        Err(EngineError::UnsupportedOption(key.to_string()))
    }

    pub fn get_option(&self, key: &str) -> Result<String, EngineError> {
        for extension in &self.extensions {
            if let Ok(ext) = extension.lock() {
                if let Ok(value) = ext.get(key) {
                    log::info!("setting read [{}]:{key}={value}", ext.name());
                    return Ok(value);
                }
            }
        }
        Err(EngineError::UnsupportedOption(key.to_string()))
    }

    pub fn options(&self) -> Vec<EngineExtensionOption> {
        let mut options = Vec::new();
        for extension in &self.extensions {
            options.extend(extension.lock().unwrap().options());
        }
        options
    }
}

impl ConfigExtension for EngineExtensionManager {
    const PREFIX: &'static str = "probing";
}

impl ExtensionOptions for EngineExtensionManager {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn cloned(&self) -> Box<dyn ExtensionOptions> {
        Box::new(EngineExtensionManager {
            extensions: self.extensions.iter().map(Arc::clone).collect(),
        })
    }

    fn set(&mut self, key: &str, value: &str) -> datafusion::error::Result<()> {
        match self.set_option(key, value) {
            Ok(_) => Ok(()),
            Err(e) => Err(datafusion::error::DataFusionError::Execution(e.to_string())),
        }
    }

    fn entries(&self) -> Vec<datafusion::config::ConfigEntry> {
        self.options()
            .iter()
            .map(|option| datafusion::config::ConfigEntry {
                key: format!("{}.{}", Self::PREFIX, option.key),
                value: option.value.clone(),
                description: option.help,
            })
            .collect()
    }
}
