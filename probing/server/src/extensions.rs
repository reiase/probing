use probing_engine::core::{EngineError, EngineExtension, EngineExtensionOption};

use crate::{start_remote, start_report_worker};

#[derive(Debug)]
pub struct ServerExtension {
    addr: Option<String>,
    unix_socket: Option<String>,
    report_addr: Option<String>,
    max_concurrent_requests: usize,
    request_timeout_ms: u64,
}

impl Default for ServerExtension {
    fn default() -> Self {
        Self {
            addr: None,
            unix_socket: None,
            report_addr: None,
            max_concurrent_requests: 100,
            request_timeout_ms: 30000,
        }
    }
}

impl EngineExtension for ServerExtension {
    fn name(&self) -> String {
        "server".to_string()
    }

    fn set(&mut self, key: &str, value: &str) -> Result<String, EngineError> {
        match key {
            "server.addr" => {
                // Validate address format
                value
                    .parse::<std::net::SocketAddr>()
                    .map_err(|_| EngineError::InvalidOption(key.to_string(), value.to_string()))?;
                if self.addr.is_some() {
                    return Err(EngineError::InvalidOption(
                        key.to_string(),
                        value.to_string(),
                    ));
                }
                let old = self.addr.clone().unwrap_or_default();
                self.addr = Some(value.to_string());
                start_remote(self.addr.clone());
                Ok(old)
            }
            "server.unix_socket" => {
                if self.unix_socket.is_some() {
                    return Err(EngineError::InvalidOption(
                        key.to_string(),
                        value.to_string(),
                    ));
                }
                let old = self.unix_socket.clone().unwrap_or_default();
                self.unix_socket = Some(value.to_string());
                Ok(old)
            }
            "server.report_addr" | "server.report.addr" => {
                if self.report_addr.is_some() {
                    return Err(EngineError::InvalidOption(
                        key.to_string(),
                        value.to_string(),
                    ));
                }
                let old = self.report_addr.clone().unwrap_or_default();
                self.report_addr = Some(value.to_string());
                start_report_worker(value.to_string(), self.addr.clone().unwrap_or_default());
                Ok(old)
            }
            "server.max_concurrent_requests" => {
                let max = value
                    .parse::<usize>()
                    .map_err(|_| EngineError::InvalidOption(key.to_string(), value.to_string()))?;
                let old = self.max_concurrent_requests.to_string();
                self.max_concurrent_requests = max;
                Ok(old)
            }
            "server.request_timeout_ms" => {
                let timeout = value
                    .parse::<u64>()
                    .map_err(|_| EngineError::InvalidOption(key.to_string(), value.to_string()))?;
                let old = self.request_timeout_ms.to_string();
                self.request_timeout_ms = timeout;
                Ok(old)
            }
            _ => Err(EngineError::UnsupportedOption(key.to_string())),
        }
    }

    fn get(&self, key: &str) -> Result<String, EngineError> {
        match key {
            "server.addr" => Ok(self.addr.clone().unwrap_or_default()),
            "server.unix_socket" => Ok(self.unix_socket.clone().unwrap_or_default()),
            "server.report_addr" | "server.report.addr" => {
                Ok(self.report_addr.clone().unwrap_or_default())
            }
            "server.max_concurrent_requests" => Ok(self.max_concurrent_requests.to_string()),
            "server.request_timeout_ms" => Ok(self.request_timeout_ms.to_string()),
            _ => Err(EngineError::UnsupportedOption(key.to_string())),
        }
    }

    fn options(&self) -> Vec<EngineExtensionOption> {
        vec![
            EngineExtensionOption {
                key: "server.addr".to_string(),
                value: self.addr.clone(),
                help: "Server bind address (e.g. 127.0.0.1:8080)",
            },
            EngineExtensionOption {
                key: "server.unix_socket".to_string(),
                value: self.unix_socket.clone(),
                help: "Unix domain socket path (e.g. /tmp/server.sock)",
            },
            EngineExtensionOption {
                key: "server.report_addr".to_string(),
                value: self.report_addr.clone(),
                help: "Report server address (e.g. 127.0.0.1:9922)",
            },
            EngineExtensionOption {
                key: "server.request_timeout_ms".to_string(),
                value: Some(self.request_timeout_ms.to_string()),
                help: "Request timeout in milliseconds",
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use probing_engine::core::EngineExtension;

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

        // Test max concurrent requests
        assert!(ext.set("server.max_concurrent_requests", "200").is_ok());
        assert_eq!(ext.get("server.max_concurrent_requests").unwrap(), "200");
        assert!(ext
            .set("server.max_concurrent_requests", "invalid")
            .is_err());

        // Test request timeout
        assert!(ext.set("server.request_timeout_ms", "5000").is_ok());
        assert_eq!(ext.get("server.request_timeout_ms").unwrap(), "5000");
        assert!(ext.set("server.request_timeout_ms", "invalid").is_err());

        // Test invalid option
        assert!(ext.set("invalid.key", "value").is_err());
        assert!(ext.get("invalid.key").is_err());

        // Test options list
        let options = ext.options();
        assert_eq!(options.len(), 3);
        assert!(options.iter().any(|opt| opt.key == "server.addr"));
        assert!(options.iter().any(|opt| opt.key == "server.unix_socket"));
        assert!(options
            .iter()
            .any(|opt| opt.key == "server.request_timeout_ms"));
    }
}
