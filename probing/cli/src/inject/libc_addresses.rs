use crate::inject::Process;
use eyre::{Context, Report, Result};
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
    /// Get the addresses of functions in the currently running process.
    fn for_current_process() -> Result<Self> {
        let mut use_libdl = false;
        let addr_of = unsafe {
            let lib = Library::new("libc.so.6")
                .wrap_err("loading local libc to get function addresses failed")?;

            move |name: &str| {
                let addr = lib
                    .get::<u64>(name.as_bytes())
                    .wrap_err(format!(
                        "getting address of symbol {name:?} from libc failed"
                    ))?
                    .into_raw() as u64;
                log::debug!("Found {name:?} at {addr:x} in libc");
                Ok::<_, Report>(addr)
            }
        };
        let addr_of_dl = unsafe {
            let lib = Library::new("libdl.so.2")
                .wrap_err("loading local libc to get function addresses failed")?;

            move |name: &str| {
                let addr = lib
                    .get::<u64>(name.as_bytes())
                    .wrap_err(format!(
                        "getting address of symbol {name:?} from libc failed"
                    ))?
                    .into_raw() as u64;
                log::debug!("Found {name:?} at {addr:x} in libdl");
                Ok::<_, Report>(addr)
            }
        };
        let dlopen_addr = match addr_of("___dlopen") {
            Ok(addr) => addr,
            Err(_) => {
                log::debug!("Could not find ___dlopen in libc, trying dlopen");
                match addr_of("dlopen") {
                    Ok(addr) => addr,
                    Err(_) => {
                        log::debug!("Could not find dlopen in libc, trying dlopen in libdl");
                        use_libdl = true;
                        addr_of_dl("dlopen")?
                    }
                }
            }
        };
        let addrs = Self {
            use_libdl: use_libdl,
            malloc: addr_of("malloc")?,
            free: addr_of("free")?,
            putenv: addr_of("putenv")?,
            setenv: addr_of("setenv")?,
            getenv: addr_of("getenv")?,
            printf: addr_of("printf")?,
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
            .wrap_err("getting current process to find the local libc offset failed")?
            .libc_address()
            .wrap_err("getting the local libc offset failed")?;
        let their_libc = process
            .libc_address()
            .wrap_err("getting the target libc offset failed")?;

        let our_libdl = Process::current()
            .wrap_err("getting current process to find the local libc offset failed")?
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
