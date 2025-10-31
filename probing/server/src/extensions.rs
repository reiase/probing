use probing_core::core::{
    EngineCall, EngineDatasource, EngineError, EngineExtension, EngineExtensionOption, Maybe,
};

use crate::{start_remote, start_report_worker};

#[derive(Debug, EngineExtension)]
pub struct ServerExtension {
    /// Server bind address (e.g. 127.0.0.1:8080)
    #[option(aliases=["addr"])]
    address: Maybe<String>,

    /// Unix domain socket path (e.g. /tmp/probing/<pid>)
    /// This option is readonly.
    #[option(aliases=["unixsocket"])]
    unix_socket: Maybe<String>,

    /// Report server address (e.g. 127.0.0.1:9922)
    #[option(aliases=["report.addr"])]
    report_addr: Maybe<String>,

    /// Authentication token for the server
    #[option(aliases=["auth.token"])]
    auth_token: Maybe<String>,

    /// Maximum number of connections allowed
    #[option(aliases=["max_conns"])]
    max_connections: Maybe<u32>,

    /// Connection timeout in seconds
    #[option(aliases=["conn_timeout"])]
    timeout: Maybe<u64>,

    /// Enable debug mode
    #[option(name = "debug")]
    debug: Maybe<bool>,

    /// Log level (trace, debug, info, warn, error)
    #[option(aliases=["loglevel"])]
    log_level: Maybe<String>,

    /// Root path for assets used by the probing UI dashboard
    #[option(aliases=["assets.root"])]
    assets_root: Maybe<String>,
}

impl EngineCall for ServerExtension {}

impl EngineDatasource for ServerExtension {}

impl Default for ServerExtension {
    fn default() -> Self {
        Self {
            address: Maybe::Nothing,
            unix_socket: Maybe::Nothing,
            report_addr: Maybe::Nothing,
            auth_token: Maybe::Nothing,
            max_connections: Maybe::Just(20), // Default to 20 connections
            timeout: Maybe::Just(30),         // Default timeout of 30 seconds
            debug: Maybe::Just(false),        // Debug mode off by default
            log_level: Maybe::Just("info".to_string()), // Default log level
            assets_root: Maybe::Nothing,
        }
    }
}

impl ServerExtension {
    fn set_address(&mut self, address: Maybe<String>) -> Result<(), EngineError> {
        let address_string: String = address.clone().into();

        // Validate address format before assignment
        address_string
            .parse::<std::net::SocketAddr>()
            .map_err(|_| {
                EngineError::InvalidOptionValue("address".to_string(), address_string.clone())
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
        //start_report_worker(report_addr_str, address_str);
        self.report_addr = report_addr;
        Ok(())
    }

    fn set_auth_token(&mut self, auth_token: Maybe<String>) -> Result<(), EngineError> {
        self.auth_token = auth_token;
        Ok(())
    }

    fn set_max_connections(&mut self, max_connections: Maybe<u32>) -> Result<(), EngineError> {
        if let Maybe::Just(count) = max_connections {
            if count == 0 {
                return Err(EngineError::InvalidOptionValue(
                    "max_connections".to_string(),
                    count.to_string(),
                ));
            }
        }
        self.max_connections = max_connections;
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
        if let Maybe::Just(ref level_str) = log_level {
            match level_str.to_lowercase().as_str() {
                "trace" | "debug" | "info" | "warn" | "error" => {
                    self.log_level = log_level;
                    Ok(())
                }
                _ => Err(EngineError::InvalidOptionValue(
                    "log_level".to_string(),
                    level_str.clone(),
                )),
            }
        } else {
            self.log_level = log_level;
            Ok(())
        }
    }

    fn set_assets_root(&mut self, assets_root: Maybe<String>) -> Result<(), EngineError> {
        self.assets_root = assets_root;
        Ok(())
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
        assert!(ext.set("addr", "127.0.0.1:8080").is_ok());
        assert_eq!(ext.get("addr").unwrap(), "127.0.0.1:8080");

        // Test invalid addr format
        assert!(ext.set("addr", "invalid").is_err());

        // Test unix socket
        assert!(ext.set("unix_socket", "/tmp/test.sock").is_ok());
        assert_eq!(ext.get("unix_socket").unwrap(), "/tmp/test.sock");

        // Test max connections
        assert!(ext.set("max_connections", "200").is_ok());
        assert_eq!(ext.get("max_connections").unwrap(), "200");
        assert!(ext.set("max_connections", "0").is_err());

        // Test timeout
        assert!(ext.set("timeout", "60").is_ok());
        assert_eq!(ext.get("timeout").unwrap(), "60");

        // Test debug mode
        assert!(ext.set("debug", "true").is_ok());
        assert_eq!(ext.get("debug").unwrap(), "true");

        // Test log level
        assert!(ext.set("log_level", "debug").is_ok());
        assert_eq!(ext.get("log_level").unwrap(), "debug");
        assert!(ext.set("log_level", "invalid").is_err());

        // Test auth token
        assert!(ext.set("auth_token", "secret123").is_ok());
        assert_eq!(ext.get("auth_token").unwrap(), "secret123");

        // Test report address
        assert!(ext.set("report_addr", "127.0.0.1:9922").is_ok());
        assert_eq!(ext.get("report_addr").unwrap(), "127.0.0.1:9922");

        // Test invalid option
        assert!(ext.set("invalid.key", "value").is_err());
        assert!(ext.get("invalid.key").is_err());

        // Test options list
        let options = ext.options();
        assert_eq!(options.len(), 9); // Updated count to include all options
        assert!(options.iter().any(|opt| opt.key == "server.address"));
        assert!(options.iter().any(|opt| opt.key == "server.unix_socket"));
        assert!(options.iter().any(|opt| opt.key == "server.report_addr"));
        assert!(options.iter().any(|opt| opt.key == "server.auth_token"));
        assert!(options
            .iter()
            .any(|opt| opt.key == "server.max_connections"));
        assert!(options.iter().any(|opt| opt.key == "server.timeout"));
        assert!(options.iter().any(|opt| opt.key == "server.debug"));
        assert!(options.iter().any(|opt| opt.key == "server.log_level"));
    }
}
