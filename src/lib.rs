#[macro_use]
extern crate ctor;

use anyhow::Result;

use probing_python::backtrace_signal_handler;
use probing_python::create_probing_module;
use probing_server::sync_env_settings;

const ENV_PROBING_LOG: &str = "PROBING_LOG";
const ENV_PROBING_PORT: &str = "PROBING_PORT";

const DEFAULT_PORT: u16 = 9700;

pub fn register_signal_handler<F>(sig: std::ffi::c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe {
        match signal_hook_registry::register_unchecked(sig, move |_: &_| handler()) {
            Ok(_) => {
                log::debug!("Registered signal handler for signal {sig}");
            }
            Err(e) => log::error!("Failed to register signal handler: {}", e),
        }
    };
}

pub fn get_hostname() -> Result<String> {
    let uname = rustix::system::uname();
    let hostname = uname.nodename();

    let ip = dns_lookup::lookup_host(hostname.to_str()?)?;
    let ip = ip
        .iter()
        .filter(|ipaddr| ipaddr.is_ipv4())
        .collect::<Vec<_>>();
    Ok(format!("{}", ip[0]))
}

#[ctor]
fn setup() {
    let pid = std::process::id();
    eprintln!("Initializing libprobing for process {pid} ...",);

    // initialize logging
    env_logger::init_from_env(env_logger::Env::new().filter(ENV_PROBING_LOG));

    // initialize signal handlers
    register_signal_handler(
        rustix::process::Signal::Usr2 as i32,
        backtrace_signal_handler,
    );

    // initialize probing server
    probing_server::start_local();

    // config remote server if port is specified
    if let Ok(port) = std::env::var(ENV_PROBING_PORT) {
        let port: u16 = port.parse().unwrap_or(DEFAULT_PORT);
        let local_rank = std::env::var("LOCAL_RANK")
            .unwrap_or("0".to_string())
            .parse()
            .unwrap_or(0);

        // determine bind address
        let hostname = if std::env::var("RANK").unwrap_or("0".to_string()) == "0" {
            "0.0.0.0".to_string()
        } else {
            get_hostname().unwrap_or("localhost".to_string())
        };

        // set server address
        let server_address = format!("'{}:{}'", hostname, port + local_rank);
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
