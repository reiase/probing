#[macro_use]
extern crate ctor;

use anyhow::Result;
use env_logger::Env;
use log::error;

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
            Ok(_) => {}
            Err(e) => error!("Failed to register signal handler: {}", e),
        }
    };
}

pub fn get_hostname() -> Result<String> {
    let uname = rustix::system::uname();
    let hostname = uname.nodename();
    Ok(hostname.to_str()?.to_string())
}

#[ctor]
fn setup() {
    let pid = std::process::id();
    eprintln!("Initializing libprobing for process {pid} ...",);
    env_logger::init_from_env(Env::new().filter(ENV_PROBING_LOG));

    register_signal_handler(
        rustix::process::Signal::Usr2 as i32,
        backtrace_signal_handler,
    );

    probing_server::start_local();

    if let Ok(port) = std::env::var(ENV_PROBING_PORT) {
        let local_rank = std::env::var("LOCAL_RANK")
            .unwrap_or("0".to_string())
            .parse()
            .unwrap_or(0);
        let hostname = if std::env::var("RANK").unwrap_or("0".to_string()) == "0" {
            "0.0.0.0".to_string()
        } else {
            get_hostname().unwrap_or("localhost".to_string())
        };

        let address = format!(
            "'{}:{}'",
            hostname,
            port.parse().unwrap_or(DEFAULT_PORT) + local_rank
        );
        std::env::set_var("PROBING_SERVER_ADDR", address);
        probing_server::start_report_worker();
    }
    let _ = create_probing_module();
    sync_env_settings();
}

#[dtor]
fn cleanup() {
    if let Err(e) = probing_server::cleanup() {
        error!("Failed to cleanup unix socket: {}", e);
    }
}
