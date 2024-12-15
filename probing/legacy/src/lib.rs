#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

use anyhow::Result;
use std::{env, ffi::c_int};

use ctrl::ctrl_handler_string;
// use env_logger::Env;
// use log::debug;
// use log::error;
use nix::libc;
// use nix::libc::SIGUSR1;
// use nix::libc::SIGUSR2;
// use server::local_server;

mod core;
pub mod ctrl;
mod handlers;
mod hooks;
mod repl;
// pub mod server;
pub mod service;

// use handlers::dump_stack2;
// use probing_proto::cli::CtrlSignal;
// use repl::PythonRepl;
// use server::remote_server;
// use server::report::start_report_worker;

pub fn register_signal_handler<F>(sig: c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe { signal_hook_registry::register_unchecked(sig, move |_: &_| handler()).unwrap() };
}

pub fn sigusr1_handler() {
    let cmdstr = env::var("PROBING_ARGS").unwrap_or("Nil".to_string());
    ctrl_handler_string(cmdstr);
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
