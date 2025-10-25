//! A library for injecting shared libraries into running processes via ptrace.
//!
//! # Platform support
//!
//! This library currently only supports x64 \*nix systems, mainly because that's
//! what I have. Support for other architectures should be possible - the only
//! barrier being that I cannot test it. In theory though, it would just be
//! a matter of re-writing the shellcode for each architecture and selecting the
//! correct one with conditional compilation.
//!
//! For Windows, use other projects like [`dll-syringe`][1].
//!
//! # Example
//!
//! ```no_run
//! use std::{process::Command, path::PathBuf};
//! use ptrace_inject::{Injector, Process};
//!
//! # fn main() -> eyre::Result<()> {
//! let library = PathBuf::from("path/to/library.so");
//!
//! // Spawn a new process and inject the library into it.
//! let target = Command::new("target-process");
//! Injector::spawn(target)?.inject(&library)?;
//!
//! // Or attach to an existing process.
//! let proc = Process::by_name("target-process")?.expect("to find target process");
//! Injector::attach(proc)?.inject(&library)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Ptrace note
//!
//! This library was inspired by [`linux-inject`][2]. As noted by that project:
//!
//! > On many Linux distributions, the kernel is configured by default to
//! > prevent any process from calling `ptrace()` on another process that it did
//! > not create (e.g. via `fork()`). This is a security feature meant to prevent
//! > exactly the kind of mischief that this tool causes. You can temporarily
//! > disable it until the next reboot using the following command:
//! > ```text
//! > echo 0 | sudo tee /proc/sys/kernel/yama/ptrace_scope
//! > ```
//!
//! This library uses [`log`][3] for logging.
//!
//!  [1]: https://crates.io/crates/dll-syringe
//!  [2]: https://github.com/gaffe23/linux-inject
//!  [3]: https://crates.io/crates/log
#![warn(clippy::all, clippy::pedantic, clippy::nursery, missing_docs)]
#![allow(
    // Errors can happen for such a diverse set of reasons out of the user's
    // control that listing them all in a form other than the variants of `Error`
    // would not be feasible or useful.
    clippy::missing_errors_doc,
    // Register names like `rsi` and `rdi` break this.
    clippy::similar_names,
)]
use anyhow::Context;
use anyhow::Result;
pub use libc_addresses::LibcAddrs;
pub use process::Process;

// Common modules
mod injection_trait;
mod libc_addresses;
mod process;

// Platform-specific injection modules
#[cfg(target_arch = "x86_64")]
mod injection;
#[cfg(target_arch = "aarch64")]
mod injection_aarch64;

/// A type capable of loading libraries into a ptrace'd target process.
///
/// When this struct is dropped it will detach from the target process.
pub struct Injector {
    /// The PID of the process we are injecting into.
    proc: Process,
    /// The tracer that is controlling the tracee.
    tracer: pete::Ptracer,
    /// All the PID/TIDs we are attached to.
    attached: Vec<nix::unistd::Pid>,
}

impl Injector {
    /// Attach to an existing process and begin tracing it.
    pub fn attach(proc: Process) -> Result<Self> {
        let mut tracer = pete::Ptracer::new();
        tracer
            .attach((&proc).into())
            .context("failed to attach to given process")?;
        log::trace!("Attached to process with PID {proc}");
        Self::new(proc, tracer)
    }

    /// Initialise a new injector by attaching to its children.
    fn new(proc: Process, tracer: pete::Ptracer) -> Result<Self> {
        let attached = vec![(&proc).into()];
        let mut injector = Self {
            proc,
            tracer,
            attached,
        };
        injector
            .attach_children()
            .context("failed to attach to child threads")?;
        Ok(injector)
    }

    /// Attach to all child threads of the process.
    fn attach_children(&mut self) -> Result<()> {
        let threads = self
            .proc
            .thread_ids()
            .context("couldn't get thread IDs of target to attach to them")?;
        log::trace!("Attaching to {} child threads of target", threads.len() - 1);
        threads
            .iter()
            .filter(|&tid| tid != &self.proc.pid())
            .try_for_each(|&tid| {
                self.tracer
                    .attach(pete::Pid::from_raw(tid))
                    .with_context(|| format!("failed to attach to child thread with TID {tid}"))?;
                self.attached.push(nix::unistd::Pid::from_raw(tid));
                // The order that the threads stop is not necessarily the same as the order
                // that they were attached to, so we don't know what tracee we're getting here.
                let actual_tid = self
                    .tracer
                    .wait()
                    .context("failed to wait for thread to stop")?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "a target thread exited quietly as soon as we started tracing it"
                        )
                    })?
                    .pid;
                log::trace!("Attached to thread ID {actual_tid} of target process");
                Ok(())
            })
    }

    /// Detach from all threads we are attached to.
    pub fn detach_children(&mut self) -> Result<()> {
        log::trace!("Detaching from {} threads", self.attached.len());
        // Pete doesn't have a wrapper for this.
        self.attached.drain(..).try_for_each(|tid| {
            nix::sys::ptrace::detach(tid, None)
                .with_context(|| format!("failed to detach from thread with TID {tid}"))
        })
    }

    /// Inject the given library into the traced process.
    pub fn inject(&mut self, library: &std::path::Path, settings: Vec<String>) -> Result<()> {
        let Some(tracee) = self.tracer.wait()? else {
            return Err(anyhow::anyhow!(
                "the target exited quietly as soon as we started tracing it"
            ));
        };
        log::trace!("Attached to process with ID {}", tracee.pid);

        // Platform-specific injection logic
        self.inject_platform_specific(library, settings, tracee)?;

        log::info!(
            "Successfully injected library {} into process with PID {}",
            library.display(),
            self.proc
        );
        Ok(())
    }

    /// Platform-specific injection implementation
    fn inject_platform_specific(
        &mut self,
        library: &std::path::Path,
        settings: Vec<String>,
        tracee: pete::Tracee,
    ) -> Result<()> {
        use injection_trait::perform_injection;

        #[cfg(target_arch = "x86_64")]
        {
            use injection::Injection;
            perform_injection::<Injection>(&self.proc, &mut self.tracer, tracee, library, settings)
                .context("failed to perform x86_64 injection")?;
        }

        #[cfg(target_arch = "aarch64")]
        {
            use injection_aarch64::InjectionAarch64;
            perform_injection::<InjectionAarch64>(
                &self.proc,
                &mut self.tracer,
                tracee,
                library,
                settings,
            )
            .context("failed to perform aarch64 injection")?;
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            return Err(anyhow::anyhow!(
                "Unsupported architecture: {}. Only x86_64 and aarch64 are supported.",
                std::env::consts::ARCH
            ));
        }

        Ok(())
    }

    // /// Put the env string into the traced process.
    // ///
    // /// # Panics
    // ///
    // /// This function may panic if it fails to inject shellcode into the target process.
    // pub fn setenv(&mut self, name: Option<&str>, value: Option<&str>) -> Result<()> {
    //     let Some(tracee) = self.tracer.wait()? else {
    //         return Err(eyre!(
    //             "the target exited quietly as soon as we started tracing it"
    //         ));
    //     };
    //     log::trace!("Attached to process with ID {}", tracee.pid);
    //     let mut injection = Injection::inject(&self.proc, &mut self.tracer, tracee)
    //         .wrap_err("failed to inject shellcode")?;

    //     injection
    //         .setenv(name, value)
    //         .wrap_err("failed to prepare env string")?;
    //     log::info!(
    //         "Successfully put env string `{}`=`{}` into process with PID {}",
    //         name.map_or("", |s| s),
    //         value.map_or("", |s| s),
    //         self.proc
    //     );
    //     injection.remove().unwrap();
    //     Ok(())
    // }
}

impl Drop for Injector {
    fn drop(&mut self) {
        log::trace!("Dropping injector");
        if let Err(e) = self.detach_children() {
            log::error!("Failed to detach from target process: {e}");
        }
    }
}
