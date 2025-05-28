use std::collections::BTreeMap;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Debug;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::config::{ConfigExtension, ExtensionOptions};
use tokio::sync::Mutex;

use super::error::EngineError;
use super::Plugin;

#[derive(Clone, Debug, Default)]
pub enum Maybe<T> {
    Just(T),
    #[default]
    Nothing,
}

impl<T: FromStr> FromStr for Maybe<T> {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(Maybe::Nothing)
        } else {
            match s.parse() {
                Ok(v) => Ok(Maybe::Just(v)),
                Err(_) => Ok(Maybe::Nothing),
            }
        }
    }
}

impl<T: Display> Display for Maybe<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Maybe::Just(s) => write!(f, "{}", s),
            Maybe::Nothing => write!(f, ""),
        }
    }
}

impl<T> From<Maybe<T>> for Option<T> {
    fn from(val: Maybe<T>) -> Self {
        match val {
            Maybe::Just(v) => Some(v),
            Maybe::Nothing => None,
        }
    }
}

impl<T: Display> From<Maybe<T>> for String {
    fn from(value: Maybe<T>) -> Self {
        match value {
            Maybe::Just(v) => v.to_string(),
            Maybe::Nothing => "".to_string(),
        }
    }
}

/// Represents a configuration option for an engine extension.
///
/// # Fields
/// * `key` - The unique identifier for this option
/// * `value` - The current value of the option, if set
/// * `help` - Static help text describing the purpose and usage of this option
pub struct EngineExtensionOption {
    pub key: String,
    pub value: Option<String>,
    pub help: &'static str,
}

/// Extension trait for handling API calls
#[allow(unused)]
#[async_trait]
pub trait EngineCall: Debug + Send + Sync {
    /// Handle API calls to the extension
    ///
    /// # Arguments
    /// * `path` - The path component of the API call
    /// * `params` - URL query parameters
    /// * `body` - Request body data
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Response data on success
    /// * `Err(EngineError)` - Error information on failure
    async fn call(
        &self,
        path: &str,
        params: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<Vec<u8>, EngineError> {
        Err(EngineError::UnsupportedCall)
    }
}

/// Extension trait for providing data sources
#[allow(unused)]
pub trait EngineDatasource: Debug + Send + Sync {
    /// Provide a data source plugin implementation
    ///
    /// # Arguments
    /// * `namespace` - The namespace for the data source
    /// * `name` - Optional name of the specific data source
    ///
    /// # Returns
    /// * `Some(Arc<dyn Plugin>)` - Data source plugin if available
    /// * `None` - If no matching data source is available
    fn datasrc(
        &self,
        namespace: &str,
        name: Option<&str>,
    ) -> Option<Arc<dyn Plugin + Sync + Send>> {
        None
    }
}

/// A trait for engine extensions that can be configured with key-value pairs.
///
/// This trait defines the interface for extensions that can be registered with
/// the [`EngineExtensionManager`] to provide configurable functionality.
///
/// # Required Methods
///
/// * [`name`] - Returns the unique name of this extension
/// * [`set`] - Sets a configuration option value
/// * [`get`] - Retrieves a configuration option value  
/// * [`options`] - Lists all available configuration options
///
/// # Examples
///
/// ```
/// use probing_core::core::{EngineExtension, EngineExtensionOption};
/// use probing_core::core::EngineCall;
/// use probing_core::core::EngineDatasource;
/// use probing_core::core::EngineError;
///
/// #[derive(Debug)]
/// struct MyExtension {
///     some_option: String
/// }
///
/// impl EngineCall for MyExtension {}
///
/// impl EngineDatasource for MyExtension {}
///
/// impl EngineExtension for MyExtension {
///     fn name(&self) -> String {
///         "my_extension".to_string()
///     }
///
///     fn set(&mut self, key: &str, value: &str) -> Result<String, EngineError> {
///         match key {
///             "some_option" => {
///                 let old = self.some_option.clone();
///                 self.some_option = value.to_string();
///                 Ok(old)
///             }
///             _ => Err(EngineError::UnsupportedOption(key.to_string()))
///         }
///     }
///
///     fn get(&self, key: &str) -> Result<String, EngineError> {
///         match key {
///             "some_option" => Ok(self.some_option.clone()),
///             _ => Err(EngineError::UnsupportedOption(key.to_string()))
///         }
///     }
///
///     fn options(&self) -> Vec<EngineExtensionOption> {
///         vec![
///             EngineExtensionOption {
///                 key: "some_option".to_string(),
///                 value: Some(self.some_option.clone()),
///                 help: "An example option"
///             }
///         ]
///     }
/// }
/// let mut ext = MyExtension { some_option: "default".to_string() };
/// assert_eq!(ext.name(), "my_extension");
/// assert_eq!(ext.set("some_option", "new").unwrap(), "default");
/// assert_eq!(ext.get("some_option").unwrap(), "new");
/// ```
#[allow(unused)]
pub trait EngineExtension: Debug + Send + Sync + EngineCall + EngineDatasource {
    fn name(&self) -> String;
    fn set(&mut self, key: &str, value: &str) -> Result<String, EngineError> {
        todo!()
    }
    fn get(&self, key: &str) -> Result<String, EngineError> {
        todo!()
    }
    fn options(&self) -> Vec<EngineExtensionOption> {
        todo!()
    }
}

/// Engine extension management module for configurable functionality.
///
/// This module provides a flexible extension system that allows for runtime configuration
/// of engine components through a key-value interface. It consists of three main components:
///
/// - [`EngineExtensionOption`]: Represents a single configuration option with metadata
/// - [`EngineExtension`]: A trait that must be implemented by configurable extensions
/// - [`EngineExtensionManager`]: Manages multiple extensions and their configurations
///
/// The extension system integrates with DataFusion's configuration framework through
/// implementations of [`ConfigExtension`] and [`ExtensionOptions`].
///
/// # Examples
///
/// ```rust
/// use std::sync::{Arc, Mutex};
/// use probing_core::core::EngineExtensionManager;
/// use probing_core::core::{EngineExtension, EngineExtensionOption};
/// use probing_core::core::EngineCall;
/// use probing_core::core::EngineDatasource;
/// use probing_core::core::EngineError;
///
/// #[derive(Debug)]
/// struct MyExtension {
///     some_option: String
/// }
///
/// impl EngineCall for MyExtension {}
///
/// impl EngineDatasource for MyExtension {}
///
/// impl EngineExtension for MyExtension {
///     fn name(&self) -> String {
///         "my_extension".to_string()
///     }
///
///     fn set(&mut self, key: &str, value: &str) -> Result<String, EngineError> {
///         match key {
///             "some_option" => {
///                 let old = self.some_option.clone();
///                 self.some_option = value.to_string();
///                 Ok(old)
///             }
///             _ => Err(EngineError::UnsupportedOption(key.to_string()))
///         }
///     }
///
///     fn get(&self, key: &str) -> Result<String, EngineError> {
///         match key {
///             "some_option" => Ok(self.some_option.clone()),
///             _ => Err(EngineError::UnsupportedOption(key.to_string()))
///         }
///     }
///
///     fn options(&self) -> Vec<EngineExtensionOption> {
///         vec![
///             EngineExtensionOption {
///                 key: "some_option".to_string(),
///                 value: Some(self.some_option.clone()),
///                 help: "An example option"
///             }
///         ]
///     }
/// }
///
/// let mut manager = EngineExtensionManager::default();
/// // Register extensions
/// manager.register(Arc::new(Mutex::new(MyExtension { some_option: "default".to_string() })));
///
/// // Configure extensions
/// manager.set_option("some_option", "new").unwrap();
/// assert_eq!(manager.get_option("some_option").unwrap(), "new");
///
/// // List all available options
/// let options = manager.options();
/// ```
#[derive(Clone, Debug, Default)]
pub struct EngineExtensionManager {
    extensions: BTreeMap<String, Arc<Mutex<dyn EngineExtension + Send + Sync>>>,
}

impl EngineExtensionManager {
    pub fn register(
        &mut self,
        name: String,
        extension: Arc<Mutex<dyn EngineExtension + Send + Sync>>,
    ) {
        self.extensions.insert(name, extension);
    }

    /// Extract namespace from extension name by removing "extension" suffix and converting to lowercase
    fn extract_namespace(extension_name: &str) -> String {
        let mut namespace = extension_name.to_lowercase();
        if namespace.ends_with("extension") {
            namespace.truncate(namespace.len() - "extension".len());
        }
        format!("{}.", namespace)
    }

    pub async fn set_option(&mut self, key: &str, value: &str) -> Result<(), EngineError> {
        for extension in self.extensions.values() {
            let mut ext = extension.lock().await;
            let namespace = Self::extract_namespace(&ext.name());
            if !key.starts_with(&namespace) {
                continue;
            }
            let local_key = key.trim_start_matches(&namespace);
            match ext.set(local_key, value) {
                Ok(old) => {
                    log::info!(
                        "setting update [{}]:{local_key}={value} <= {old}",
                        ext.name()
                    );
                    return Ok(());
                }
                Err(EngineError::UnsupportedOption(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(EngineError::UnsupportedOption(key.to_string()))
    }

    pub async fn get_option(&self, key: &str) -> Result<String, EngineError> {
        for extension in self.extensions.values() {
            let ext = extension.lock().await;
            let namespace = Self::extract_namespace(&ext.name());
            if !key.starts_with(&namespace) {
                continue;
            }
            let local_key = key.trim_start_matches(&namespace);
            if let Ok(value) = ext.get(local_key) {
                log::info!("setting read [{}]:{local_key}={value}", ext.name());
                return Ok(value);
            }
        }
        Err(EngineError::UnsupportedOption(key.to_string()))
    }

    pub async fn options(&self) -> Vec<EngineExtensionOption> {
        let mut all_options = Vec::new();
        for extension_arc in self.extensions.values() {
            let ext_guard = extension_arc.lock().await;
            all_options.extend(ext_guard.options());
        }
        all_options
    }

    pub async fn call(
        &self,
        path: &str,
        params: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<Vec<u8>, EngineError> {
        for extension in self.extensions.values() {
            let ext = extension.lock().await;
            let name = ext.name();
            log::debug!("checking extension [{}]:{}", name, path);

            let expected_prefix = format!("/{}/", name);
            if !path.starts_with(&expected_prefix) {
                continue;
            }

            let local_path = path[expected_prefix.len()..].to_string();
            match ext.call(&local_path, params, body).await {
                Ok(value) => return Ok(value),
                Err(EngineError::UnsupportedCall) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(EngineError::CallError(path.to_string()))
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
            extensions: self
                .extensions
                .iter()
                .map(|(name, ext)| (name.clone(), ext.clone()))
                .collect(),
        })
    }

    fn set(&mut self, key: &str, value: &str) -> datafusion::error::Result<()> {
        let key_owned = key.to_string();
        let value_owned = value.to_string();

        use futures::executor::block_on;

        block_on(async { self.set_option(&key_owned, &value_owned).await })
            .map_err(|e| datafusion::error::DataFusionError::Execution(e.to_string()))
    }

    fn entries(&self) -> Vec<datafusion::config::ConfigEntry> {
        use futures::executor::block_on;

        block_on(async {
            self.options()
                .await
                .iter()
                .map(|option| datafusion::config::ConfigEntry {
                    key: format!("{}.{}", Self::PREFIX, option.key),
                    value: option.value.clone(),
                    description: option.help,
                })
                .collect()
        })
    }
}
