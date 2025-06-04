#[macro_use]
extern crate ctor;

use anyhow::Result;

use probing_python::create_probing_module;
use probing_server::sync_env_settings;

const ENV_PROBING_LOGLEVEL: &str = "PROBING_LOGLEVEL";
const ENV_PROBING_PORT: &str = "PROBING_PORT";

const DEFAULT_PORT: u16 = 9700;

#[cfg(feature = "use-mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub fn get_hostname() -> Result<String> {
    let ips = nix::ifaddrs::getifaddrs()?
        .filter_map(|addr| addr.address)
        .filter_map(|addr| addr.as_sockaddr_in().cloned())
        .filter_map(|addr| {
            let ip_addr = addr.ip();
            match ip_addr.is_unspecified() {
                true => None,
                false => Some(ip_addr.to_string()),
            }
        })
        .collect::<Vec<_>>();

    // Check for address pattern match from environment variable
    if let Ok(pattern) = std::env::var("PROBING_SERVER_ADDRPATTERN") {
        for ip in ips.iter() {
            if ip.starts_with(pattern.as_str()) {
                log::debug!("Select IP address {ip} with pattern {pattern}");
                return Ok(ip.clone());
            }
            log::debug!("Skip IP address {ip} with pattern {pattern}");
        }
    }

    // Return first IP if no pattern match found
    ips.first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("No suitable IP address found"))
}

#[ctor]
fn setup() {
    let pid = std::process::id();
    eprintln!("Initializing libprobing for process {pid} ...",);

    // initialize logging
    env_logger::init_from_env(env_logger::Env::new().filter(ENV_PROBING_LOGLEVEL));

    // initialize probing server
    probing_server::start_local();

    // config remote server if port is specified
    if let Ok(port) = std::env::var(ENV_PROBING_PORT) {
        let port: u16 = port.parse().unwrap_or(DEFAULT_PORT);
        let local_rank = std::env::var("LOCAL_RANK")
            .unwrap_or("0".to_string())
            .parse()
            .unwrap_or(0);

        let serving_port = port + local_rank;
        log::debug!("serving on port {serving_port} for local rank {local_rank}.");

        // determine bind address
        let hostname = if std::env::var("RANK").unwrap_or("0".to_string()) == "0" {
            "0.0.0.0".to_string()
        } else {
            get_hostname().unwrap_or("localhost".to_string())
        };

        // set server address
        let server_address = format!("'{}:{}'", hostname, serving_port);
        std::env::set_var("PROBING_SERVER_ADDR", server_address);

        // set report address if master exists
        if let Ok(master_addr) = std::env::var("MASTER_ADDR") {
            std::env::set_var(
                "PROBING_SERVER_REPORT_ADDR",
                format!("'{}:{}'", master_addr, port),
            );
        }
    }

    // initialize probing python module
    let _ = create_probing_module();
    sync_env_settings();
}

#[dtor]
fn cleanup() {
    if let Err(e) = probing_server::cleanup() {
        log::error!("Failed to cleanup unix socket: {}", e);
    }
}
