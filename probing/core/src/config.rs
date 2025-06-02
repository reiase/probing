use std::collections::HashMap;

use crate::core::{EngineError, EngineExtensionManager};
use crate::ENGINE;

/// Global configuration management interface that provides unified access
/// to the engine extension manager from any process.
///
/// This module exposes the EngineExtensionManager as a unified configuration
/// management interface, allowing any part of the codebase to read/write
/// configuration settings through the engine extension system.
///
/// # Usage Examples
///
/// ```rust
/// # async fn usage_example() -> Result<(), probing_core::core::EngineError> {
/// // Note: These examples assume the probing engine is initialized appropriately.
/// // In a test environment without full engine setup, operations requiring
/// // an initialized engine might return `EngineError::EngineNotInitialized`.
///
/// // Set a configuration option
/// probing_core::config::set("server.address", "127.0.0.1:8080").await?;
///
/// // Get a configuration option
/// let addr = probing_core::config::get("server.address").await?;
/// // For a test, you might assert the value:
/// // assert_eq!(addr, "127.0.0.1:8080");
///
/// // List all available configuration options
/// let options = probing_core::config::list_options().await;
/// // `options` will be empty if the engine is not initialized or has no config.
///
/// // Check if engine is initialized
/// if probing_core::config::is_engine_initialized().await {
///     println!("Engine is ready for configuration");
/// } else {
///     println!("Engine is not initialized.");
/// }
/// # Ok(())
/// # }
/// ```

/// Set a configuration option through the engine extension system.
///
/// This function finds the appropriate extension that handles the given key
/// and updates its configuration. The change takes effect immediately.
///
/// # Arguments
/// * `key` - The configuration option key (e.g., "server.address", "torch.profiling_mode")
/// * `value` - The new value for the configuration option
///
/// # Returns
/// * `Ok(())` - Configuration was successfully updated
/// * `Err(EngineError)` - Configuration update failed (invalid key, value, or engine not initialized)
///
/// # Examples
/// ```rust
/// # async fn example() -> Result<(), probing_core::core::EngineError> {
/// // These calls assume the probing engine is initialized.
/// // If not, they may return `EngineError::EngineNotInitialized`.
///
/// // Set server address
/// probing_core::config::set("server.address", "0.0.0.0:8080").await?;
///
/// // Set profiling interval
/// probing_core::config::set("taskstats.interval", "1000").await?;
///
/// // Enable debug mode
/// probing_core::config::set("server.debug", "true").await?;
/// # Ok(())
/// # }
/// ```
pub async fn set(key: &str, value: &str) -> Result<(), EngineError> {
    let engine_guard = ENGINE.write().await;
    let mut state = engine_guard.context.state();

    // Get a mutable reference to the extension manager
    if let Some(eem) = state
        .config_mut()
        .options_mut()
        .extensions
        .get_mut::<EngineExtensionManager>()
    {
        eem.set_option(key, value).await?; // The EngineExtensionManager handles the option setting.

        // Note: The EngineExtensionManager is responsible for applying this specific option.
        // Broader engine re-configuration, if necessary based on this change,
        // would be handled by the engine's internal logic after this call.
        log::info!(
            "Configuration option processed via EngineExtensionManager: {} = {}",
            key,
            value
        );
        Ok(())
    } else {
        Err(EngineError::EngineNotInitialized)
    }
}

/// Get a configuration option through the engine extension system.
///
/// This function queries all registered extensions to find the one that
/// handles the given key and returns its current value.
///
/// # Arguments
/// * `key` - The configuration option key to retrieve
///
/// # Returns
/// * `Ok(String)` - The current value of the configuration option
/// * `Err(EngineError)` - Key not found or engine not initialized
///
/// # Examples
/// ```rust
/// # async fn example() -> Result<(), probing_core::core::EngineError> {
/// // Get server address
/// let addr = probing_core::config::get("server.address").await?;
///
/// // Get current profiling mode
/// let mode = probing_core::config::get("torch.profiling_mode").await?;
/// # Ok(())
/// # }
/// ```
pub async fn get(key: &str) -> Result<String, EngineError> {
    let engine = ENGINE.read().await;
    let state = engine.context.state();

    if let Some(eem) = state
        .config()
        .options()
        .extensions
        .get::<EngineExtensionManager>()
    {
        eem.get_option(key).await
    } else {
        Err(EngineError::EngineNotInitialized)
    }
}

/// List all available configuration options from all registered extensions.
///
/// This function aggregates configuration options from all registered extensions,
/// providing a comprehensive view of what can be configured in the system.
///
/// # Returns
/// * `Vec<EngineExtensionOption>` - List of all available configuration options
///
/// # Examples
/// ```rust
/// # async fn example() -> Result<(), probing_core::core::EngineError> {
/// let options = probing_core::config::list_options().await;
/// for option in options {
///     println!("{}: {} ({})", option.key,
///              option.value.unwrap_or_default(), option.help);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn list_options() -> Vec<crate::core::EngineExtensionOption> {
    let engine = ENGINE.read().await;
    let state = engine.context.state();

    if let Some(eem) = state
        .config()
        .options()
        .extensions
        .get::<EngineExtensionManager>()
    {
        eem.options().await
    } else {
        Vec::new()
    }
}

/// Get all configuration options as a HashMap for easy programmatic access.
///
/// This is a convenience method that returns all current configuration values
/// in a HashMap format, making it easy to iterate over or lookup specific values.
///
/// # Returns
/// * `HashMap<String, String>` - Map of all configuration keys to their current values
///
/// # Examples
/// ```rust
/// # async fn example() -> Result<(), probing_core::core::EngineError> {
/// use probing_core::config::get_all;
/// let config_map = get_all().await;
/// for (key, value) in config_map {
///     println!("{} = {}", key, value);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn get_all() -> HashMap<String, String> {
    let mut config_map = HashMap::new();
    let options = list_options().await;

    for option in options {
        if let Some(value) = option.value {
            config_map.insert(option.key, value);
        }
    }

    config_map
}

/// Check if the engine is initialized and ready for configuration operations.
///
/// This function verifies that the global ENGINE is properly initialized
/// and has an accessible EngineExtensionManager.
///
/// # Returns
/// * `true` - Engine is initialized and ready for configuration
/// * `false` - Engine is not yet initialized
///
/// # Examples
/// ```rust
/// # async fn example() -> Result<(), probing_core::core::EngineError> {
/// if probing_core::config::is_engine_initialized().await {
///     probing_core::config::set("server.address", "0.0.0.0:8080").await?;
///     println!("Engine initialized and config set.");
/// } else {
///     println!("Engine not yet initialized");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn is_engine_initialized() -> bool {
    let engine = ENGINE.read().await;
    let state = engine.context.state();
    state
        .config()
        .options()
        .extensions
        .get::<EngineExtensionManager>()
        .is_some()
}

/// Make an API call to a specific extension.
///
/// This function routes API calls to the appropriate extension based on the path.
/// Extensions can implement custom API endpoints for advanced functionality.
///
/// # Arguments
/// * `path` - API path (e.g., "/server/status", "/pprof/profile")
/// * `params` - Query parameters as key-value pairs
/// * `body` - Request body data
///
/// # Returns
/// * `Ok(Vec<u8>)` - Response data from the extension
/// * `Err(EngineError)` - API call failed or extension not found
///
/// # Examples
/// ```rust
/// # use std::collections::HashMap;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let params = HashMap::new();
/// // This call assumes the engine and relevant extension are initialized.
/// let response = probing_core::config::call_extension("/server/status", &params, &[]).await?;
/// let status = String::from_utf8(response)?;
/// println!("Status: {}", status);
/// # Ok(())
/// # }
/// ```
pub async fn call_extension(
    path: &str,
    params: &HashMap<String, String>,
    body: &[u8],
) -> Result<Vec<u8>, EngineError> {
    let engine = ENGINE.read().await;
    let state = engine.context.state();

    if let Some(eem) = state
        .config()
        .options()
        .extensions
        .get::<EngineExtensionManager>()
    {
        eem.call(path, params, body).await
    } else {
        Err(EngineError::EngineNotInitialized)
    }
}

/// Set multiple configuration options at once.
///
/// This is a convenience method for bulk configuration updates. It attempts
/// to set all provided options and returns the first error encountered, if any.
///
/// # Arguments
/// * `options` - HashMap of configuration keys to values
///
/// # Returns
/// * `Ok(())` - All options were successfully set
/// * `Err(EngineError)` - At least one option failed to set
///
/// # Examples
/// ```rust
/// # use std::collections::HashMap;
/// # async fn example() -> Result<(), probing_core::core::EngineError> {
/// let mut options = HashMap::new();
/// options.insert("server.address".to_string(), "0.0.0.0:8080".to_string());
/// options.insert("server.debug".to_string(), "true".to_string());
/// probing_core::config::set_multiple(&options).await?;
/// # Ok(())
/// # }
/// ```
pub async fn set_multiple(options: &HashMap<String, String>) -> Result<(), EngineError> {
    for (key, value) in options {
        set(key, value).await?;
    }
    Ok(())
}

/// Get multiple configuration options at once.
///
/// This is a convenience method for bulk configuration retrieval. It returns
/// a HashMap with the requested keys and their values. Keys that don't exist
/// or can't be retrieved are omitted from the result.
///
/// # Arguments
/// * `keys` - List of configuration keys to retrieve
///
/// # Returns
/// * `HashMap<String, String>` - Map of successfully retrieved configuration options
///
/// # Examples
/// ```rust
/// # async fn example() -> Result<(), probing_core::core::EngineError> {
/// use probing_core::config::get_multiple;
/// let keys = vec!["server.address", "server.debug"];
/// let values = get_multiple(&keys).await;
/// // Process `values` HashMap...
/// # Ok(())
/// # }
/// ```
pub async fn get_multiple(keys: &[&str]) -> HashMap<String, String> {
    let mut result = HashMap::new();

    for key in keys {
        if let Ok(value) = get(key).await {
            result.insert(key.to_string(), value);
        }
    }

    result
}

/// Environment variable integration utilities.
///
/// These functions help bridge between traditional environment variables
/// and the unified configuration system.
pub mod env {
    use super::*;

    /// Sync an environment variable to a configuration option.
    ///
    /// This function reads an environment variable and sets the corresponding
    /// configuration option if the environment variable exists.
    ///
    /// # Arguments
    /// * `env_var` - Environment variable name
    /// * `config_key` - Configuration option key
    ///
    /// # Returns
    /// * `Ok(true)` - Environment variable was found and configuration was updated
    /// * `Ok(false)` - Environment variable was not found, no change made
    /// * `Err(EngineError)` - Configuration update failed
    ///
    /// # Examples
    /// ```rust
    /// # async fn example() -> Result<(), probing_core::core::EngineError> {
    /// // Ensure the "SERVER_ADDRESS" env var is set for this example to have an effect.
    /// // std::env::set_var("SERVER_ADDRESS", "127.0.0.1_from_env");
    /// let synced = probing_core::config::env::sync_env_to_config("SERVER_ADDRESS", "server.address").await?;
    /// if synced {
    ///     println!("Synced SERVER_ADDRESS to config");
    /// } else {
    ///     println!("SERVER_ADDRESS not found in environment.");
    /// }
    /// // std::env::remove_var("SERVER_ADDRESS"); // Clean up
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sync_env_to_config(env_var: &str, config_key: &str) -> Result<bool, EngineError> {
        if let Ok(value) = std::env::var(env_var) {
            super::set(config_key, &value).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Sync a configuration option to an environment variable.
    ///
    /// This function reads a configuration option and sets the corresponding
    /// environment variable if the configuration option exists.
    ///
    /// # Arguments
    /// * `config_key` - Configuration option key
    /// * `env_var` - Environment variable name
    ///
    /// # Returns
    /// * `Ok(true)` - Configuration was found and environment variable was set
    /// * `Ok(false)` - Configuration was not found, no change made
    /// * `Err(EngineError)` - Configuration retrieval failed
    ///
    /// # Examples
    /// ```rust
    /// # async fn example() -> Result<(), probing_core::core::EngineError> {
    /// // First, ensure the config "server.address" has a value if testing actual sync.
    /// // probing_core::config::set("server.address", "example.com:8080").await?;
    /// let synced = probing_core::config::env::sync_config_to_env("server.address", "SERVER_ADDRESS_OUT").await?;
    /// if synced {
    ///     // In a test: assert_eq!(std::env::var("SERVER_ADDRESS_OUT").unwrap(), "example.com:8080");
    ///     println!("Synced server.address to SERVER_ADDRESS_OUT env var");
    /// } else {
    ///     println!("Config server.address not found or other issue.");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sync_config_to_env(config_key: &str, env_var: &str) -> Result<bool, EngineError> {
        match super::get(config_key).await {
            Ok(value) => {
                std::env::set_var(env_var, value);
                Ok(true)
            }
            Err(EngineError::UnsupportedOption(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Sync multiple environment variables to configuration options.
    ///
    /// This function takes a mapping of environment variable names to configuration
    /// keys and syncs all of them.
    ///
    /// # Arguments
    /// * `mappings` - HashMap of environment variable names to configuration keys
    ///
    /// # Returns
    /// * `HashMap<String, bool>` - Map of environment variable names to sync success status
    ///
    /// # Examples
    /// ```rust
    /// # use std::collections::HashMap;
    /// # async fn example() -> Result<(), probing_core::core::EngineError> {
    /// let mut mappings = HashMap::new();
    /// mappings.insert("SERVER_ADDRESS_ENV".to_string(), "server.address.conf".to_string());
    /// mappings.insert("SERVER_DEBUG_ENV".to_string(), "server.debug.conf".to_string());
    /// // For testing, you might set these env vars:
    /// // std::env::set_var("SERVER_ADDRESS_ENV", "env_addr");
    /// // std::env::set_var("SERVER_DEBUG_ENV", "true_from_env");
    /// let results = probing_core::config::env::sync_multiple_env_to_config(&mappings).await;
    /// // Process `results` HashMap, e.g., check `results.get("SERVER_ADDRESS_ENV")`
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sync_multiple_env_to_config(
        mappings: &HashMap<String, String>,
    ) -> HashMap<String, bool> {
        let mut results = HashMap::new();

        for (env_var, config_key) in mappings {
            let success = sync_env_to_config(env_var, config_key)
                .await
                .unwrap_or(false);
            results.insert(env_var.clone(), success);
        }

        results
    }

    /// Get all environment variables that match a prefix pattern.
    ///
    /// This utility function helps identify environment variables that should
    /// be mapped to configuration options.
    ///
    /// # Arguments
    /// * `prefix` - Prefix to match (e.g., "PROBING_", "SERVER_")
    ///
    /// # Returns
    /// * `HashMap<String, String>` - Map of environment variable names to values
    ///
    /// # Examples
    /// ```rust
    /// # use std::collections::HashMap;
    /// # fn example() { // This function is not async
    /// // For testing, you might set these env vars:
    /// // std::env::set_var("PROBING_VAR1", "val1");
    /// // std::env::set_var("PROBING_ANOTHER", "val2");
    /// let probing_vars = probing_core::config::env::get_env_vars_with_prefix("PROBING_");
    /// // for (key, value) in probing_vars {
    /// //     println!("Env: {}={}", key, value);
    /// // }
    /// // assert!(probing_vars.contains_key("PROBING_VAR1"));
    /// // std::env::remove_var("PROBING_VAR1"); // Clean up
    /// // std::env::remove_var("PROBING_ANOTHER"); // Clean up
    /// # }
    /// ```
    pub fn get_env_vars_with_prefix(prefix: &str) -> HashMap<String, String> {
        std::env::vars()
            .filter(|(key, _)| key.starts_with(prefix))
            .collect()
    }
}
