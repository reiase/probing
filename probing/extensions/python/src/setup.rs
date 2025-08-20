use nix::sys::signal::{self, SaFlags, SigAction, SigHandler, SigSet};

pub fn register_signal_handler<F>(sig: std::ffi::c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe {
        match signal_hook_registry::register_unchecked(sig, move |_: &_| handler()) {
            Ok(_) => {
                log::debug!("Registered signal handler for signal {sig}");
            }
            Err(e) => log::error!("Failed to register signal handler: {e}"),
        }
    };
}

fn register_segvsignal_handler() -> nix::Result<()> {
    let sa = SigAction::new(
        SigHandler::Handler(crate::features::stack_tracer::exit_segvsignal_handler),
        SaFlags::SA_RESTART,  
        SigSet::empty()       
    );
    
    unsafe{
        signal::sigaction(signal::SIGSEGV, &sa)?;
    }
    
    Ok(())
}

#[ctor]
fn setup() {
    register_signal_handler(
        nix::libc::SIGUSR2,
        crate::features::stack_tracer::backtrace_signal_handler,
    );
    register_signal_handler(
        nix::libc::SIGTERM,
        crate::features::stack_tracer::exit_signal_handler,
    );
    register_signal_handler(
        nix::libc::SIGUSR1,
        crate::features::stack_tracer::exit_signal_handler,
    );
    register_segvsignal_handler();
}
