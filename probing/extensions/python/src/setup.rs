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

#[ctor]
fn setup() {
    register_signal_handler(
        nix::libc::SIGUSR2,
        crate::features::stack_tracer::backtrace_signal_handler,
    );
    crate::features::pprof::setup(100);
}
