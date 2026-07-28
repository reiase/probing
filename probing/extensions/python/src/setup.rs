#[cfg(probing_snapshot_unwind)]
use nix::sys::signal::{self, SaFlags, SigAction, SigHandler, SigSet};

#[cfg(probing_snapshot_unwind)]
fn register_backtrace_signal_handler() -> nix::Result<()> {
    let action = SigAction::new(
        SigHandler::SigAction(crate::features::stack_tracer::backtrace_signal_handler),
        SaFlags::SA_RESTART | SaFlags::SA_SIGINFO,
        SigSet::empty(),
    );

    unsafe {
        signal::sigaction(signal::SIGUSR2, &action)?;
    }
    Ok(())
}

#[ctor]
fn setup() {
    #[cfg(probing_snapshot_unwind)]
    match register_backtrace_signal_handler() {
        Ok(()) => log::debug!("Registered SIGUSR2 stack snapshot handler"),
        Err(error) => log::error!("Failed to register SIGUSR2 handler: {error}"),
    }

    #[cfg(not(probing_snapshot_unwind))]
    log::debug!("SIGUSR2 stack snapshots are unsupported on this platform");
}
