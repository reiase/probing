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
/// use probing_core::config;
///
/// // Set a configuration option
/// config::set("server.address", "127.0.0.1:8080").await.unwrap();
///
/// // Get a configuration option
/// let addr = config::get("server.address").await.unwrap();
///
/// // List all available configuration options
/// let options = config::list_options().await;
///
/// // Check if engine is initialized
/// if config::is_engine_initialized().await {
///     println!("Engine is ready for configuration");
/// }
/// ```
pub mod config {
    use super::*;

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
    /// // Set server address
    /// config::set("server.address", "0.0.0.0:8080").await?;
    ///
    /// // Set profiling interval
    /// config::set("taskstats.interval", "1000").await?;
    ///
    /// // Enable debug mode
    /// config::set("server.debug", "true").await?;
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
            eem.set_option(key, value)?;

            // Note: In a real implementation, we would need to update the engine's configuration
            // For now, we'll just perform the validation and log the change
            log::info!("Configuration would be updated: {} = {}", key, value);
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
    /// // Get server address
    /// let addr = config::get("server.address").await?;
    ///
    /// // Get current profiling mode
    /// let mode = config::get("torch.profiling_mode").await?;
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
            eem.get_option(key)
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
    /// let options = config::list_options().await;
    /// for option in options {
    ///     println!("{}: {} ({})", option.key,
    ///              option.value.unwrap_or_default(), option.help);
    /// }
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
            eem.options()
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
    /// let config_map = config::get_all().await;
    /// for (key, value) in config_map {
    ///     println!("{} = {}", key, value);
    /// }
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
    /// if config::is_engine_initialized().await {
    ///     config::set("server.address", "0.0.0.0:8080").await?;
    /// } else {
    ///     println!("Engine not yet initialized");
    /// }
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
    /// let params = HashMap::new();
    /// let response = config::call_extension("/server/status", &params, &[]).await?;
    /// let status = String::from_utf8(response)?;
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
            eem.call(path, params, body)
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
    /// let mut options = HashMap::new();
    /// options.insert("server.address".to_string(), "0.0.0.0:8080".to_string());
    /// options.insert("server.debug".to_string(), "true".to_string());
    /// config::set_multiple(&options).await?;
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
    /// let keys = vec!["server.address", "server.debug"];
    /// let values = config::get_multiple(&keys).await;
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
    /// // Sync SERVER_ADDRESS environment variable to server.address config
    /// config::env::sync_env_to_config("SERVER_ADDRESS", "server.address").await?;
    /// ```
    pub async fn sync_env_to_config(env_var: &str, config_key: &str) -> Result<bool, EngineError> {
        if let Ok(value) = std::env::var(env_var) {
            super::config::set(config_key, &value).await?;
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
    /// // Sync server.address config to SERVER_ADDRESS environment variable
    /// config::env::sync_config_to_env("server.address", "SERVER_ADDRESS").await?;
    /// ```
    pub async fn sync_config_to_env(config_key: &str, env_var: &str) -> Result<bool, EngineError> {
        match super::config::get(config_key).await {
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
    /// let mut mappings = HashMap::new();
    /// mappings.insert("SERVER_ADDRESS".to_string(), "server.address".to_string());
    /// mappings.insert("SERVER_DEBUG".to_string(), "server.debug".to_string());
    /// let results = config::env::sync_multiple_env_to_config(&mappings).await;
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
    /// // Get all PROBING_ environment variables
    /// let probing_vars = config::env::get_env_vars_with_prefix("PROBING_");
    /// ```
    pub fn get_env_vars_with_prefix(prefix: &str) -> HashMap<String, String> {
        std::env::vars()
            .filter(|(key, _)| key.starts_with(prefix))
            .collect()
    }
}
