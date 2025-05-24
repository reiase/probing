use probing_core::core::{
    EngineCall, EngineDatasource, EngineError, EngineExtension, EngineExtensionOption, Maybe,
};

use crate::{start_remote, start_report_worker};

#[derive(Debug, EngineExtension)]
pub struct ServerExtension {
    /// Server bind address (e.g. 127.0.0.1:8080)
    #[option(name = "server.address", aliases=["server_address", "server.addr", "server_addr"])]
    address: Maybe<String>,

    /// Unix domain socket path (e.g. /tmp/probing/<pid>)
    /// This option is readonly.
    #[option(name = "server.unix_socket", aliases=["server_unix_socket", "server.unixsocket"])]
    unix_socket: Maybe<String>,

    /// Report server address (e.g. 127.0.0.1:9922)
    #[option(name = "server.report_addr", aliases=["server_report_addr", "server.report.addr"])]
    report_addr: Maybe<String>,
    
    /// Maximum number of connections allowed
    #[option(name = "server.max_connections", aliases=["server_max_connections", "server.max_conns"])]
    max_connections: Maybe<u32>,
    
    /// Connection timeout in seconds
    #[option(name = "server.timeout", aliases=["server_timeout", "server.conn_timeout"])]
    timeout: Maybe<u64>,
    
    /// Enable debug mode
    #[option(name = "server.debug", aliases=["server_debug"])]
    debug: Maybe<bool>,
    
    /// Log level (trace, debug, info, warn, error)
    #[option(name = "server.log_level", aliases=["server_log_level", "server.loglevel"])]
    log_level: Maybe<String>,
}

impl EngineCall for ServerExtension {}

impl EngineDatasource for ServerExtension {}

impl Default for ServerExtension {
    fn default() -> Self {
        Self {
            address: Maybe::Nothing,
            unix_socket: Maybe::Nothing,
            report_addr: Maybe::Nothing,
            max_connections: Maybe::Just(20), // Default to 100 connections
            timeout: Maybe::Just(30),          // Default timeout of 30 seconds
            debug: Maybe::Just(false),         // Debug mode off by default
            log_level: Maybe::Just("info".to_string()), // Default log level
        }
    }
}

impl ServerExtension {
    fn set_address(&mut self, address: Maybe<String>) -> Result<(), EngineError> {
        let address_string: String = address.clone().into();
        address_string.parse::<std::net::SocketAddr>().map_err(|_| {
            EngineError::InvalidOptionValue("server.address".to_string(), address_string.clone())
        })?;
        self.address = address;
        start_remote(address_string.into());
        Ok(())
    }

    fn set_unix_socket(&mut self, unix_socket: Maybe<String>) -> Result<(), EngineError> {
        self.unix_socket = unix_socket;
        Ok(())
    }

    fn set_report_addr(&mut self, report_addr: Maybe<String>) -> Result<(), EngineError> {
        let report_addr_str: String = report_addr.clone().into();
        let address_str: String = self.address.clone().into();
        start_report_worker(report_addr_str, address_str);
        self.report_addr = report_addr;
        Ok(())
    }
    
    fn set_max_connections(&mut self, max_connections: Maybe<u32>) -> Result<(), EngineError> {
        match max_connections {
            Maybe::Nothing => {
                self.max_connections = Maybe::Nothing;
            }
            Maybe::Just(count) => {
                if count == 0 {
                    return Err(EngineError::InvalidOptionValue(
                        "server.max_connections".to_string(),
                        "0".to_string(),
                    ));
                }
                self.max_connections = max_connections;
            }    
        }
        Ok(())
    }
    
    fn set_timeout(&mut self, timeout: Maybe<u64>) -> Result<(), EngineError> {
        self.timeout = timeout;
        Ok(())
    }
    
    fn set_debug(&mut self, debug: Maybe<bool>) -> Result<(), EngineError> {
        self.debug = debug;
        Ok(())
    }
    
    fn set_log_level(&mut self, log_level: Maybe<String>) -> Result<(), EngineError> {
        let level_str: String = log_level.clone().into();
        match level_str.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {
                self.log_level = log_level;
                Ok(())
            }
            _ => Err(EngineError::InvalidOptionValue(
                "server.log_level".to_string(),
                level_str,
            )),
        }
    }

    // Public getters for Config integration
    pub fn get_address(&self) -> &Maybe<String> {
        &self.address
    }
    
    pub fn get_unix_socket(&self) -> &Maybe<String> {
        &self.unix_socket
    }
    
    pub fn get_report_addr(&self) -> &Maybe<String> {
        &self.report_addr
    }
    
    pub fn get_max_connections(&self) -> &Maybe<u32> {
        &self.max_connections
    }
    
    pub fn get_timeout(&self) -> &Maybe<u64> {
        &self.timeout
    }
    
    pub fn get_debug(&self) -> &Maybe<bool> {
        &self.debug
    }
    
    pub fn get_log_level(&self) -> &Maybe<String> {
        &self.log_level
    }
}

#[cfg(test)]
mod test {
    use probing_core::core::EngineExtension;

    use crate::extensions::ServerExtension;

    #[test]
    fn test_server_extension() {
        let mut ext = ServerExtension::default();

        // Test setting and getting addr
        assert!(ext.set("server.addr", "127.0.0.1:8080").is_ok());
        assert_eq!(ext.get("server.addr").unwrap(), "127.0.0.1:8080");

        // Test invalid addr format
        assert!(ext.set("server.addr", "invalid").is_err());

        // Test unix socket
        assert!(ext.set("server.unix_socket", "/tmp/test.sock").is_ok());
        assert_eq!(ext.get("server.unix_socket").unwrap(), "/tmp/test.sock");
        
        // Test max connections
        assert!(ext.set("server.max_connections", "200").is_ok());
        assert_eq!(ext.get("server.max_connections").unwrap(), "200");
        assert!(ext.set("server.max_connections", "0").is_err());
        
        // Test timeout
        assert!(ext.set("server.timeout", "60").is_ok());
        assert_eq!(ext.get("server.timeout").unwrap(), "60");
        
        // Test debug mode
        assert!(ext.set("server.debug", "true").is_ok());
        assert_eq!(ext.get("server.debug").unwrap(), "true");
        
        // Test log level
        assert!(ext.set("server.log_level", "debug").is_ok());
        assert_eq!(ext.get("server.log_level").unwrap(), "debug");
        assert!(ext.set("server.log_level", "invalid").is_err());

        // Test invalid option
        assert!(ext.set("invalid.key", "value").is_err());
        assert!(ext.get("invalid.key").is_err());

        // Test options list
        let options = ext.options();
        assert_eq!(options.len(), 7); // Updated count to include new options
        assert!(options.iter().any(|opt| opt.key == "server.address"));
        assert!(options.iter().any(|opt| opt.key == "server.unix_socket"));
        assert!(options.iter().any(|opt| opt.key == "server.max_connections"));
        assert!(options.iter().any(|opt| opt.key == "server.timeout"));
        assert!(options.iter().any(|opt| opt.key == "server.debug"));
        assert!(options.iter().any(|opt| opt.key == "server.log_level"));
    }
}
