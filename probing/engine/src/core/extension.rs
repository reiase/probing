use std::sync::Arc;

use super::error::EngineError;

pub struct ExtensionOption {
    key: String,
    value: Option<String>,
    help: &'static str,
}

pub trait EngineExtension {
    fn set(&self, key: &str, value: &str) -> Result<(), EngineError>;
    fn get(&self, key: &str) -> Result<String, EngineError>;
    fn options(&self) -> Vec<ExtensionOption>;
}
pub struct EngineExtensionManager {
    extensions: Vec<Arc<dyn EngineExtension>>,
}

impl EngineExtensionManager {
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
        }
    }

    pub fn register(&mut self, extension: Arc<dyn EngineExtension>) {
        self.extensions.push(extension);
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), EngineError> {
        for extension in &self.extensions {
            match extension.set(key, value) {
                Ok(_) => return Ok(()),
                Err(_) => continue,
            }
        }
        Err(EngineError::UnsupportedOption(key.to_string()))
    }

    pub fn get(&self, key: &str) -> Result<String, EngineError> {
        for extension in &self.extensions {
            if let Ok(value) = extension.get(key) {
                return Ok(value);
            }
        }
        Err(EngineError::UnsupportedOption(key.to_string()))
    }

    pub fn options(&self) -> Vec<ExtensionOption> {
        let mut options = Vec::new();
        for extension in &self.extensions {
            options.extend(extension.options());
        }
        options
    }
}
