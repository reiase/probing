#[macro_use]
extern crate ctor;

use anyhow::Result;
use env_logger::Env;
use log::error;
use nix::libc;
use nix::libc::SIGUSR2;

use probing_python::backtrace_signal_handler;
use probing_python::create_probing_module;

const ENV_PROBING_LOG: &str = "PROBING_LOG";
const ENV_PROBING_ARGS: &str = "PROBING_ARGS";
const ENV_PROBING_PORT: &str = "PROBING_PORT";

const DEFAULT_PORT: u16 = 9700;

pub fn register_signal_handler<F>(sig: std::ffi::c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe { signal_hook_registry::register_unchecked(sig, move |_: &_| handler()).unwrap() };
}

pub fn get_hostname() -> Result<String> {
    let limit = unsafe { libc::sysconf(libc::_SC_HOST_NAME_MAX) };
    let size = libc::c_long::max(limit, 256) as usize;

    // Reserve additional space for terminating nul byte.
    let mut buffer = vec![0u8; size + 1];

    #[allow(trivial_casts)]
    let result = unsafe { libc::gethostname(buffer.as_mut_ptr() as *mut libc::c_char, size) };

    if result != 0 {
        return Err(anyhow::anyhow!("gethostname failed"));
    }

    let hostname = std::ffi::CStr::from_bytes_until_nul(buffer.as_slice())?;

    Ok(hostname.to_str()?.to_string())
}

#[ctor]
fn setup() {
    let pid = std::process::id();
    eprintln!("Initializing libprobing for process {pid} ...",);
    env_logger::init_from_env(Env::new().filter(ENV_PROBING_LOG));

    register_signal_handler(SIGUSR2, backtrace_signal_handler);

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
            "{}:{}",
            hostname,
            port.parse().unwrap_or(DEFAULT_PORT) + local_rank
        );
        println!(
            "Starting remote server for process {} at {}",
            std::process::id(),
            address
        );
        probing_server::start_remote(Some(address));
        probing_server::start_report_worker();
    }
    let _ = create_probing_module();
}

#[dtor]
fn cleanup() {
    if let Err(e) = probing_server::cleanup() {
        error!("Failed to cleanup unix socket: {}", e);
    }
}
