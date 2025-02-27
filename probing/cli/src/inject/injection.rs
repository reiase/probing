use crate::inject::{LibcAddrs, Process};
// use eyre::{eyre, Context, Result};
use anyhow::Context;
use anyhow::Result;
use std::os::unix::ffi::OsStringExt;

/// The x64 shellcode that will be injected into the tracee.
const SHELLCODE: [u8; 6] = [
    // Nop slide to make up for the fact that jumping is imprecise.
    0x90, 0x90, // nop; nop
    // The tracer does most of the work by putting the arguments into the
    // relevant registers, and the function pointer into `r9`.
    0x41, 0xff, 0xd1, // call r9
    // Trap so that the tracer can set up the next call.
    0xcc, // int3
];

/// A type for managing the injection, execution and removal of shellcode in a
/// target process (tracee).
#[derive(Debug)]
pub struct Injection<'a> {
    /// The state of the tracee's registers before the injection.
    saved_registers: pete::Registers,
    /// The original state of the memory that was overwritten by the injection.
    saved_memory: Vec<u8>,
    /// The address at which the shellcode was injected.
    injected_at: u64,
    /// The addresses within the tracee's address space of the libc functions
    /// that we need.
    libc: LibcAddrs,
    /// The tracer that is controlling the tracee.
    tracer: &'a mut pete::Ptracer,
    /// The process we are injecting into.
    tracee: pete::Tracee,
    /// If the injection is not explicitly removed, we attempt to do so when
    /// it is dropped, but we need to know whether or not it was already removed.
    removed: bool,
}

impl<'a> Injection<'a> {
    /// Inject the shellcode into the given tracee.
    pub(crate) fn inject(
        proc: &Process,
        tracer: &'a mut pete::Ptracer,
        mut tracee: pete::Tracee,
    ) -> Result<Self> {
        let injected_at = proc
            .find_executable_space()
            .context("couldn't find region to write shellcode")?;
        log::debug!("Injecting shellcode at {injected_at:x}");
        let saved_memory = tracee
            .read_memory(injected_at, SHELLCODE.len())
            .context("failed to read memory we were going to overwrite")?;
        log::trace!("Read memory to overwrite: {saved_memory:x?}");
        tracee
            .write_memory(injected_at, &SHELLCODE)
            .context("failed to write shellcode to tracee")?;
        log::trace!("Written shellcode");
        let saved_registers = tracee
            .registers()
            .context("failed to save original tracee registers")?;
        log::trace!("Saved registers: {saved_registers:x?}");
        let libc = LibcAddrs::for_process(proc)
            .context("couldn't get libc function addresses for tracee")?;
        log::trace!("Found libc addresses: {libc:x?}");
        log::debug!("Injected shellcode into tracee");
        Ok(Self {
            saved_registers,
            saved_memory,
            injected_at,
            libc,
            tracer,
            tracee,
            removed: false,
        })
    }

    /// Use the injected shellcode to load the library at the given path.
    pub(crate) fn execute(&mut self, filename: &std::path::Path) -> Result<()> {
        let address = self
            .write_filename(filename)
            .context("couldn't write library filename to tracee address space")?;
        self.open_library(address)
            .context("failed to load library in tracee")?;
        self.free_alloc(address)
            .context("failed to free memory we stored the library filename in")?;
        log::debug!("Executed injected shellcode to load library");
        Ok(())
    }

    /// Allocate space for, and write, a filename in the tracee's address space.
    ///
    /// Returns the address of the filename.
    fn write_filename(&mut self, filename: &std::path::Path) -> Result<u64> {
        // Get the absolute path since the tracee's CWD could be anything.
        let mut filename = std::fs::canonicalize(filename)
            .context("couldn't get absolute path of given library")?
            .into_os_string()
            .into_vec();
        // Null-terminate the filename.
        filename.push(0);
        // RSI is unused, 0 is arbitrary.
        let address = self
            .call_function(self.libc.malloc, filename.len() as u64, 0)
            .context("calling malloc in tracee failed")?;
        if address == 0 {
            return Err(anyhow::anyhow!("malloc within tracee returned NULL"));
        }
        log::trace!(
            "Allocated {} bytes at {address:x} in tracee for library filename",
            filename.len(),
        );
        self.tracee
            .write_memory(address, &filename)
            .context("writing library name to tracee failed")?;
        log::debug!("Wrote library filename to tracee");
        Ok(address)
    }

    /// Open a library in the tracee, where the library's filename is already
    /// stored in the tracee's address space, at `filename_address`.
    fn open_library(&mut self, filename_address: u64) -> Result<()> {
        let result = self
            .call_function(self.libc.dlopen, filename_address, 1) // flags = RTLD_LAZY
            .context("calling dlopen in tracee failed")?;
        log::debug!("Called dlopen in tracee, result = {result:x}");
        if result == 0 {
            Err(anyhow::anyhow!("dlopen within tracee returned NULL"))
        } else {
            Ok(())
        }
    }

    /// Free memory allocated in the tracee.
    fn free_alloc(&mut self, address: u64) -> Result<()> {
        // Apparently RSI has to be 0 or free might try to free that address too?
        let result = self
            .call_function(self.libc.free, address, 0)
            .context("calling free in tracee failed")?;
        log::debug!("Freed memory in tracee, result = {result:x}");
        // Freeing is an optional cleanup step, don't check the result.
        Ok(())
    }

    pub(crate) fn setenv(&mut self, name: Option<&str>, value: Option<&str>) -> Result<()> {
        if let (Some(name), Some(value)) = (name, value) {
            let name_address = self
                .write_str(name)
                .context("failed to allocate memory for env name")?;
            let value_address = self
                .write_str(value)
                .context("failed to allocate memory for env value")?;
            let _ = self.call_function3(self.libc.setenv, name_address, value_address, 1);
            self.free_alloc(name_address)
                .context("failed to free memory storing the env name")?;
            self.free_alloc(value_address)
                .context("failed to free memory storing the env value")?;
        }
        Ok(())
    }

    /// Allocate space for, and write, a string in the tracee's address space.
    ///
    /// Return the address of the string.
    fn write_str(&mut self, s: &str) -> Result<u64> {
        let mut s = s.as_bytes().to_vec();
        s.push(0);
        let address = self
            .call_function(self.libc.malloc, s.len() as u64, 0)
            .context("calling malloc in tracee failed")?;
        if address == 0 {
            return Err(anyhow::anyhow!("malloc within tracee returned NULL"));
        }
        log::trace!(
            "Allocated {} bytes at {address:x} in tracee for env string",
            s.len(),
        );
        self.tracee
            .write_memory(address, &s)
            .context("writing env str to tracee failed")?;
        log::debug!("Wrote env str to tracee");
        Ok(address)
    }

    /// Make a function call in the tracee via the injected shellcode.
    fn call_function(&mut self, fn_address: u64, rdi: u64, rsi: u64) -> Result<u64> {
        log::trace!("Calling function at {fn_address:x} with rdi = {rdi:x}, rsi = {rsi:x}");
        self.tracee
            .set_registers(pete::Registers {
                // Jump to the start of the shellcode. `rip` seems to be
                // decremented when the tracee is resumed, so we make up for that.
                rip: self.injected_at + 2,
                // The shellcode calls whatever is pointed to by `r9`.
                r9: fn_address,
                // The relevant functions seem to take their arguments in these
                // registers.
                rdi,
                rsi,
                // Ensure that the stack pointer is aligned to a 16 byte boundary, as required by
                // the x86-64 ABI.
                rsp: self.saved_registers.rsp & !0xf,
                ..self.saved_registers
            })
            .context("setting tracee registers to run shellcode failed")?;
        self.run_until_trap()
            .context("waiting for shellcode in tracee to trap failed")?;
        let result = self
            .tracee
            .registers()
            .context("reading shellcode call result from tracee registers failed")?
            .rax;
        log::trace!("Function returned {result:x}");
        Ok(result)
    }

    /// Make a function call with 3 arguments
    fn call_function3(&mut self, fn_address: u64, rdi: u64, rsi: u64, rdx: u64) -> Result<u64> {
        log::trace!("Calling function at {fn_address:x} with rdi = {rdi:x}, rsi = {rsi:x}");
        self.tracee
            .set_registers(pete::Registers {
                // Jump to the start of the shellcode. `rip` seems to be
                // decremented when the tracee is resumed, so we make up for that.
                rip: self.injected_at + 2,
                // The shellcode calls whatever is pointed to by `r9`.
                r9: fn_address,
                // The relevant functions seem to take their arguments in these
                // registers.
                rdi,
                rsi,
                rdx,
                // Ensure that the stack pointer is aligned to a 16 byte boundary, as required by
                // the x86-64 ABI.
                rsp: self.saved_registers.rsp & !0xf,
                ..self.saved_registers
            })
            .context("setting tracee registers to run shellcode failed")?;
        self.run_until_trap()
            .context("waiting for shellcode in tracee to trap failed")?;
        let result = self
            .tracee
            .registers()
            .context("reading shellcode call result from tracee registers failed")?
            .rax;
        log::trace!("Function returned {result:x}");
        Ok(result)
    }

    /// Run the tracee until it reaches a trap instruction.
    fn run_until_trap(&mut self) -> Result<()> {
        log::trace!("Running tracee until it receives a signal");
        self.tracer
            .restart(self.tracee, pete::Restart::Continue)
            .context("resuming tracee to wait for trap failed")?;
        while let Some(tracee) = self
            .tracer
            .wait()
            .context("waiting for tracee trap failed")?
        {
            log::trace!("Tracee stopped with {:?}", tracee.stop);
            match tracee.stop {
                pete::Stop::SignalDelivery {
                    signal: pete::Signal::SIGTRAP,
                } => {
                    self.tracee = tracee;
                    return Ok(());
                }
                pete::Stop::SignalDelivery { signal } | pete::Stop::Group { signal } => {
                    let rip = tracee.registers().unwrap().rip;
                    return Err(anyhow::anyhow!(
                        "shellcode running in tracee sent unexpected signal {signal:?} at rip={rip:x}",
                    ));
                }
                _ => {
                    log::trace!("Not an interesting stop, continuing running tracee");
                    self.tracer
                        .restart(tracee, pete::Restart::Continue)
                        .context("re-resuming tracee to wait for trap failed")?;
                }
            };
        }
        Err(anyhow::anyhow!(
            "tracee exited while we were waiting for trap"
        ))
    }

    /// Remove the injected shellcode and restore the tracee to its original
    /// state.
    pub(crate) fn remove(mut self) -> Result<()> {
        self.remove_internal()
    }

    /// `remove` doesn't *need* to consume self, it only does so because the
    /// instance shouldn't be used after it's been removed. This private method
    /// implements the actual removal, and is also used by the `Drop` impl.
    fn remove_internal(&mut self) -> Result<()> {
        if self.removed {
            log::trace!("Already removed injection, doing nothing");
            return Ok(());
        }
        self.tracee
            .write_memory(self.injected_at, &self.saved_memory)
            .context("restoring original code to tracee failed")?;
        log::trace!("Restored memory the injection overwrote");
        self.tracee
            .set_registers(self.saved_registers)
            .context("restoring original registers to tracee failed")?;
        log::trace!("Restored tracee registers");
        log::debug!("Removed injection");
        self.removed = true;
        Ok(())
    }
}

impl Drop for Injection<'_> {
    fn drop(&mut self) {
        if !self.removed {
            log::warn!("Injection dropped without being removed, removing now");
        }
        if let Err(e) = self
            .remove_internal()
            .context("removing injection from drop impl failed")
        {
            log::error!("Failed to remove injection: {e}");
        };
    }
}
