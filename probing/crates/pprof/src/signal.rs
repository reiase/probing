use nix::libc;

use signal_hook_registry::register_unchecked;

pub type SignalHandler = unsafe extern "C" fn();

static mut PPROF_SIGNAL_HANDLER: Option<SignalHandler> = None;

fn perf_signal_handler(_siginfo: &libc::siginfo_t) {
    log::debug!("running pprof signal handler");
    unsafe {
        if let Some(handler) = PPROF_SIGNAL_HANDLER {
            handler()
        }
    }
}

pub fn set_pprof_signal_handler(handler: SignalHandler) {
    unsafe {
        PPROF_SIGNAL_HANDLER = Some(handler);
    }
}

pub fn get_pprof_signal_handler() -> Option<SignalHandler> {
    unsafe { PPROF_SIGNAL_HANDLER }
}

#[ctor]
fn setup() {
    log::debug!("setup pprof signal handler");
    unsafe {
        register_unchecked(libc::SIGPROF, perf_signal_handler);
    }
}
