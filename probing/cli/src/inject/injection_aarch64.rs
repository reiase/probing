use crate::inject::injection_trait::InjectionTrait;
use crate::inject::{LibcAddrs, Process};
use anyhow::Context;
use anyhow::Result;
use std::os::unix::ffi::OsStringExt;

/// The aarch64 shellcode that will be injected into the tracee.
///
/// This shellcode:
/// 1. Calls the function pointed to by x8 (function address)
/// 2. Uses x0, x1, x2, x3 as arguments (first 4 arguments)
/// 3. Traps with BRK instruction for the tracer to handle
const SHELLCODE_AARCH64: [u8; 16] = [
    // NOP instructions for alignment and safety
    0x1f, 0x20, 0x03, 0xd5, // nop
    0x1f, 0x20, 0x03, 0xd5, // nop
    // Call the function pointed to by x8
    0x00, 0x01, 0x3f, 0xd6, // blr x8
    // Trap instruction for the tracer
    0x00, 0x00, 0x20, 0xd4, // brk #0
];

/// A type for managing the injection, execution and removal of shellcode in a
/// target process (tracee) on aarch64 platforms.
#[derive(Debug)]
pub struct InjectionAarch64<'a> {
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

impl<'a> InjectionAarch64<'a> {
    /// Inject the shellcode into the given tracee.
    pub(crate) fn inject(
        proc: &Process,
        tracer: &'a mut pete::Ptracer,
        mut tracee: pete::Tracee,
    ) -> Result<Self> {
        let injected_at = proc
            .find_executable_space()
            .context("couldn't find region to write shellcode")?;
        log::debug!("Injecting aarch64 shellcode at {injected_at:x}");
        let saved_memory = tracee
            .read_memory(injected_at, SHELLCODE_AARCH64.len())
            .context("failed to read memory we were going to overwrite")?;
        log::trace!("Read memory to overwrite: {saved_memory:x?}");
        tracee
            .write_memory(injected_at, &SHELLCODE_AARCH64)
            .context("failed to write shellcode to tracee")?;
        log::trace!("Written aarch64 shellcode");
        let saved_registers = tracee
            .registers()
            .context("failed to save original tracee registers")?;
        log::trace!("Saved registers: {saved_registers:x?}");
        let libc = LibcAddrs::for_process(proc)
            .context("couldn't get libc function addresses for tracee")?;
        log::trace!("Found libc addresses: {libc:x?}");
        log::debug!("Injected aarch64 shellcode into tracee");
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
        log::debug!("Executed injected aarch64 shellcode to load library");
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
        // x1 is unused, 0 is arbitrary.
        let address = self
            .call_function(self.libc.malloc, filename.len() as u64, 0, 0)
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
            .call_function(self.libc.dlopen, filename_address, 1, 0) // flags = RTLD_LAZY
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
        // x1, x2, x3 are unused for free
        let result = self
            .call_function(self.libc.free, address, 0, 0)
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
            let _ = self.call_function4(self.libc.setenv, name_address, value_address, 1, 0);
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
            .call_function(self.libc.malloc, s.len() as u64, 0, 0)
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
    ///
    /// On aarch64, the calling convention uses:
    /// - x0, x1, x2, x3 for the first 4 arguments
    /// - x8 for the function address
    /// - x0 for the return value
    fn call_function(&mut self, fn_address: u64, x0: u64, x1: u64, x2: u64) -> Result<u64> {
        log::trace!(
            "Calling function at {fn_address:x} with x0 = {x0:x}, x1 = {x1:x}, x2 = {x2:x}"
        );
        self.tracee
            .set_registers(pete::Registers {
                // Jump to the start of the shellcode
                pc: self.injected_at,
                // The shellcode calls whatever is pointed to by x8
                x8: fn_address,
                // The relevant functions take their arguments in these registers
                x0,
                x1,
                x2,
                // Ensure that the stack pointer is aligned to a 16 byte boundary, as required by
                // the aarch64 ABI
                sp: self.saved_registers.sp & !0xf,
                ..self.saved_registers
            })
            .context("setting tracee registers to run shellcode failed")?;
        self.run_until_trap()
            .context("waiting for shellcode in tracee to trap failed")?;
        let result = self
            .tracee
            .registers()
            .context("reading shellcode call result from tracee registers failed")?
            .x0;
        log::trace!("Function returned {result:x}");
        Ok(result)
    }

    /// Make a function call with 4 arguments
    fn call_function4(
        &mut self,
        fn_address: u64,
        x0: u64,
        x1: u64,
        x2: u64,
        x3: u64,
    ) -> Result<u64> {
        log::trace!("Calling function at {fn_address:x} with x0 = {x0:x}, x1 = {x1:x}, x2 = {x2:x}, x3 = {x3:x}");
        self.tracee
            .set_registers(pete::Registers {
                // Jump to the start of the shellcode
                pc: self.injected_at,
                // The shellcode calls whatever is pointed to by x8
                x8: fn_address,
                // The relevant functions take their arguments in these registers
                x0,
                x1,
                x2,
                x3,
                // Ensure that the stack pointer is aligned to a 16 byte boundary, as required by
                // the aarch64 ABI
                sp: self.saved_registers.sp & !0xf,
                ..self.saved_registers
            })
            .context("setting tracee registers to run shellcode failed")?;
        self.run_until_trap()
            .context("waiting for shellcode in tracee to trap failed")?;
        let result = self
            .tracee
            .registers()
            .context("reading shellcode call result from tracee registers failed")?
            .x0;
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
                    let pc = tracee.registers().unwrap().pc;
                    return Err(anyhow::anyhow!(
                        "shellcode running in tracee sent unexpected signal {signal:?} at pc={pc:x}",
                    ));
                }
                _ => {
                    log::trace!("Not an interesting stop, continuing running tracee");
                    self.tracer
                        .restart(tracee, pete::Restart::Continue)
                        .context("re-resuming tracee to wait for trap failed")?;
                }
            }
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
        log::debug!("Removed aarch64 injection");
        self.removed = true;
        Ok(())
    }
}

impl Drop for InjectionAarch64<'_> {
    fn drop(&mut self) {
        if !self.removed {
            log::warn!("Aarch64 injection dropped without being removed, removing now");
        }
        if let Err(e) = self
            .remove_internal()
            .context("removing aarch64 injection from drop impl failed")
        {
            log::error!("Failed to remove aarch64 injection: {e}");
        }
    }
}

impl InjectionTrait for InjectionAarch64<'_> {
    fn inject(
        proc: &crate::inject::Process,
        tracer: &mut pete::Ptracer,
        tracee: pete::Tracee,
    ) -> Result<Self> {
        Self::inject(proc, tracer, tracee)
    }

    fn execute(&mut self, filename: &std::path::Path) -> Result<()> {
        self.execute(filename)
    }

    fn setenv(&mut self, name: Option<&str>, value: Option<&str>) -> Result<()> {
        self.setenv(name, value)
    }

    fn remove(self) -> Result<()> {
        self.remove()
    }
}
