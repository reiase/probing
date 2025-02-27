use crate::inject::Process;
use anyhow::Context;
use anyhow::Result;
use libloading::os::unix::Library;

/// The address of the libc functions we need to call within a process.
#[derive(Debug, Clone, Copy)]
pub struct LibcAddrs {
    pub(crate) use_libdl: bool,
    pub(crate) malloc: u64,
    pub(crate) dlopen: u64,
    pub(crate) free: u64,
    pub(crate) putenv: u64,
    pub(crate) setenv: u64,
    pub(crate) getenv: u64,
    pub(crate) printf: u64,
}

impl LibcAddrs {
    fn find_symbol(lib: &Library, name: &str) -> Result<u64> {
        let addr = unsafe {
            lib.get::<u64>(name.as_bytes())
                .with_context(|| format!("getting address of symbol {name:?} failed"))?
                .into_raw() as u64
        };
        log::debug!("Found {name:?} at {addr:x}");
        Ok(addr)
    }

    fn addr_of(lib: &str, name: &str) -> Result<u64> {
        let lib = unsafe {
            Library::new(lib)
                .with_context(|| format!("loading lib {lib} to get function addresses failed"))?
        };

        Self::find_symbol(&lib, name)
    }

    fn find_dlopen() -> Result<(bool, u64)> {
        if let Ok(addr) = Self::addr_of("libc.so.6", "___dlopen") {
            return Ok((false, addr));
        };
        if let Ok(addr) = Self::addr_of("libc.so.6", "dlopen") {
            return Ok((false, addr));
        };
        if let Ok(addr) = Self::addr_of("libdl.so.2", "dlopen") {
            return Ok((true, addr));
        };
        anyhow::bail!("Could not find dlopen in libc or libdl")
    }

    /// Get the addresses of functions in the currently running process.
    fn for_current_process() -> Result<Self> {
        let (use_libdl, dlopen_addr) = Self::find_dlopen()?;
        let addrs = Self {
            use_libdl: use_libdl,
            malloc: Self::addr_of("libc.so.6", "malloc")?,
            free: Self::addr_of("libc.so.6", "free")?,
            putenv: Self::addr_of("libc.so.6", "putenv")?,
            setenv: Self::addr_of("libc.so.6", "setenv")?,
            getenv: Self::addr_of("libc.so.6", "getenv")?,
            printf: Self::addr_of("libc.so.6", "printf")?,
            dlopen: dlopen_addr,
        };
        log::debug!("Got libc addresses for current process: {addrs:x?}");
        Ok(addrs)
    }

    /// Given the offset of libc in the process that this struct has addresses
    /// for, and the offset of libc in another process, return the addresses of
    /// the same functions in the other process.
    const fn change_base(
        &self,
        old_base: u64,
        new_base: u64,
        old_dl: Option<u64>,
        new_dl: Option<u64>,
    ) -> Self {
        // We cannot calculate an offset as `new_base - old_base` because it
        // might be less than 0.
        let dlopen = match (old_dl, new_dl) {
            (Some(old), Some(new)) if self.use_libdl => self.dlopen - old + new,
            _ => self.dlopen - old_base + new_base,
        };
        Self {
            use_libdl: self.use_libdl,
            malloc: self.malloc - old_base + new_base,
            dlopen,
            free: self.free - old_base + new_base,
            putenv: self.putenv - old_base + new_base,
            setenv: self.setenv - old_base + new_base,
            getenv: self.getenv - old_base + new_base,
            printf: self.printf - old_base + new_base,
        }
    }

    /// Get the addresses of functions in a given process - the whole point.
    pub(crate) fn for_process(process: &Process) -> Result<Self> {
        let our_libc = Process::current()
            .context("getting current process to find the local libc offset failed")?
            .libc_address()
            .context("getting the local libc offset failed")?;
        let their_libc = process
            .libc_address()
            .context("getting the target libc offset failed")?;

        let our_libdl = Process::current()
            .context("getting current process to find the local libc offset failed")?
            .libdl_address()
            .ok();
        let their_libdl = process.libdl_address().ok();
        log::debug!(
            "Calculating libc address given our offset {:x} and their offset {:x}",
            our_libc,
            their_libc
        );
        let addrs =
            Self::for_current_process()?.change_base(our_libc, their_libc, our_libdl, their_libdl);
        log::debug!(
            "Got libc addresses for process {}: {addrs:x?}",
            process.pid()
        );
        Ok(addrs)
    }
}
