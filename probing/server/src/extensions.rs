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
}

impl EngineCall for ServerExtension {}

impl EngineDatasource for ServerExtension {}

impl Default for ServerExtension {
    fn default() -> Self {
        Self {
            address: Maybe::Nothing,
            unix_socket: Maybe::Nothing,
            report_addr: Maybe::Nothing,
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

        // Test invalid option
        assert!(ext.set("invalid.key", "value").is_err());
        assert!(ext.get("invalid.key").is_err());

        // Test options list
        let options = ext.options();
        assert_eq!(options.len(), 3);
        assert!(options.iter().any(|opt| opt.key == "server.address"));
        assert!(options.iter().any(|opt| opt.key == "server.unix_socket"));
    }
}
