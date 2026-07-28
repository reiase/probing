#[cfg(not(probing_snapshot_unwind))]
use anyhow::Result;
#[cfg(not(probing_snapshot_unwind))]
use nix::libc;

#[cfg(probing_snapshot_unwind)]
mod supported {
    use std::io;
    use std::mem::{size_of, zeroed};
    use std::os::raw::{c_char, c_int, c_void};
    use std::ptr;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Once;
    use std::time::{Duration, Instant};

    use anyhow::{anyhow, Result};
    use nix::libc;
    use once_cell::sync::Lazy;

    const MAX_FRAMES: usize = 512;
    const POLL_INTERVAL_MS: i32 = 20;
    const SNAPSHOT_MAGIC: u32 = 0x5052_4254;
    const SNAPSHOT_VERSION: u16 = 1;
    const SNAPSHOT_STACK_SIZE: usize = 2048;
    const SNAPSHOT_REGISTER_COUNT: usize = 17;

    const UNW_X86_64_RAX: usize = 0;
    const UNW_X86_64_RDX: usize = 1;
    const UNW_X86_64_RCX: usize = 2;
    const UNW_X86_64_RBX: usize = 3;
    const UNW_X86_64_RSI: usize = 4;
    const UNW_X86_64_RDI: usize = 5;
    const UNW_X86_64_RBP: usize = 6;
    const UNW_X86_64_RSP: usize = 7;
    const UNW_X86_64_R8: usize = 8;
    const UNW_X86_64_R9: usize = 9;
    const UNW_X86_64_R10: usize = 10;
    const UNW_X86_64_R11: usize = 11;
    const UNW_X86_64_R12: usize = 12;
    const UNW_X86_64_R13: usize = 13;
    const UNW_X86_64_R14: usize = 14;
    const UNW_X86_64_R15: usize = 15;
    const UNW_X86_64_RIP: usize = 16;

    const UNW_REG_IP: c_int = UNW_X86_64_RIP as c_int;
    const UNW_CACHE_NONE: c_int = 0;
    const UNW_EUNSPEC: c_int = 1;
    const UNW_ENOMEM: c_int = 2;
    const UNW_EBADREG: c_int = 3;
    const UNW_EREADONLYREG: c_int = 4;
    const UNW_EINVAL: c_int = 8;

    static BACKTRACE_MUTEX: Lazy<tokio::sync::Mutex<()>> =
        Lazy::new(|| tokio::sync::Mutex::new(()));
    static PIPE_READ_FD: AtomicI32 = AtomicI32::new(-1);
    static PIPE_WRITE_FD: AtomicI32 = AtomicI32::new(-1);
    static PIPE_INIT: Once = Once::new();
    static HANDLER_ERROR: AtomicI32 = AtomicI32::new(0);
    static HANDLER_ACTIVE: AtomicI32 = AtomicI32::new(0);

    #[repr(C)]
    struct SnapshotPrefix {
        magic: u32,
        version: u16,
        register_count: u16,
        stack_size: u32,
        reserved: u32,
        stack_base: u64,
        registers: [u64; SNAPSHOT_REGISTER_COUNT],
    }

    #[repr(C)]
    struct Snapshot {
        prefix: SnapshotPrefix,
        stack: [u8; SNAPSHOT_STACK_SIZE],
    }

    impl Snapshot {
        fn zeroed() -> Self {
            // Snapshot contains only integer fields, so an all-zero value is valid.
            unsafe { zeroed() }
        }

        fn is_valid(&self) -> bool {
            self.prefix.magic == SNAPSHOT_MAGIC
                && self.prefix.version == SNAPSHOT_VERSION
                && self.prefix.register_count as usize == SNAPSHOT_REGISTER_COUNT
                && self.prefix.stack_size as usize == SNAPSHOT_STACK_SIZE
        }
    }

    const _: () = assert!(size_of::<Snapshot>() < 4096);

    type UnwWord = u64;
    type UnwAddrSpace = *mut c_void;
    type FindProcInfo =
        unsafe extern "C" fn(UnwAddrSpace, UnwWord, *mut c_void, c_int, *mut c_void) -> c_int;
    type PutUnwindInfo = unsafe extern "C" fn(UnwAddrSpace, *mut c_void, *mut c_void);
    type GetDynInfoListAddr =
        unsafe extern "C" fn(UnwAddrSpace, *mut UnwWord, *mut c_void) -> c_int;
    type AccessMem =
        unsafe extern "C" fn(UnwAddrSpace, UnwWord, *mut UnwWord, c_int, *mut c_void) -> c_int;
    type AccessReg =
        unsafe extern "C" fn(UnwAddrSpace, c_int, *mut UnwWord, c_int, *mut c_void) -> c_int;
    type AccessFpreg =
        unsafe extern "C" fn(UnwAddrSpace, c_int, *mut c_void, c_int, *mut c_void) -> c_int;
    type Resume = unsafe extern "C" fn(UnwAddrSpace, *mut UnwCursor, *mut c_void) -> c_int;
    type GetProcName = unsafe extern "C" fn(
        UnwAddrSpace,
        UnwWord,
        *mut c_char,
        usize,
        *mut UnwWord,
        *mut c_void,
    ) -> c_int;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct UnwAccessors {
        find_proc_info: Option<FindProcInfo>,
        put_unwind_info: Option<PutUnwindInfo>,
        get_dyn_info_list_addr: Option<GetDynInfoListAddr>,
        access_mem: Option<AccessMem>,
        access_reg: Option<AccessReg>,
        access_fpreg: Option<AccessFpreg>,
        resume: Option<Resume>,
        get_proc_name: Option<GetProcName>,
    }

    #[repr(C)]
    struct UnwCursor {
        opaque: [UnwWord; 127],
    }

    struct UnwindContext {
        snapshot: *const Snapshot,
        local_accessors: *const UnwAccessors,
    }

    extern "C" {
        #[link_name = "_Ux86_64_local_addr_space"]
        static mut UNW_LOCAL_ADDR_SPACE: UnwAddrSpace;
        #[link_name = "_Ux86_64_create_addr_space"]
        fn unw_create_addr_space(accessors: *mut UnwAccessors, byte_order: c_int) -> UnwAddrSpace;
        #[link_name = "_Ux86_64_destroy_addr_space"]
        fn unw_destroy_addr_space(address_space: UnwAddrSpace);
        #[link_name = "_Ux86_64_get_accessors"]
        fn unw_get_accessors(address_space: UnwAddrSpace) -> *mut UnwAccessors;
        #[link_name = "_Ux86_64_set_caching_policy"]
        fn unw_set_caching_policy(address_space: UnwAddrSpace, policy: c_int) -> c_int;
        #[link_name = "_Ux86_64_init_remote"]
        fn unw_init_remote(
            cursor: *mut UnwCursor,
            address_space: UnwAddrSpace,
            argument: *mut c_void,
        ) -> c_int;
        #[link_name = "_Ux86_64_get_reg"]
        fn unw_get_reg(cursor: *mut UnwCursor, register: c_int, value: *mut UnwWord) -> c_int;
        #[link_name = "_Ux86_64_step"]
        fn unw_step(cursor: *mut UnwCursor) -> c_int;
    }

    /// The SIGUSR2 handler only copies register/stack bytes into a nonblocking
    /// pipe. It performs no formatting, allocation, logging, locking, or unwind.
    pub extern "C" fn backtrace_signal_handler(
        signal: c_int,
        info: *mut libc::siginfo_t,
        context: *mut c_void,
    ) {
        let _ = signal;
        let _ = info;

        if HANDLER_ACTIVE
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            HANDLER_ERROR.store(libc::EBUSY, Ordering::Release);
            return;
        }

        let mut error = 0;
        let fd = PIPE_WRITE_FD.load(Ordering::Acquire);
        if fd < 0 || context.is_null() {
            error = if fd < 0 { libc::EBADF } else { libc::EINVAL };
        } else {
            let ucontext = unsafe { &*context.cast::<libc::ucontext_t>() };
            let mut prefix = SnapshotPrefix {
                magic: SNAPSHOT_MAGIC,
                version: SNAPSHOT_VERSION,
                register_count: SNAPSHOT_REGISTER_COUNT as u16,
                stack_size: SNAPSHOT_STACK_SIZE as u32,
                reserved: 0,
                stack_base: 0,
                registers: [0; SNAPSHOT_REGISTER_COUNT],
            };

            prefix.registers[UNW_X86_64_RAX] =
                ucontext.uc_mcontext.gregs[libc::REG_RAX as usize] as u64;
            prefix.registers[UNW_X86_64_RDX] =
                ucontext.uc_mcontext.gregs[libc::REG_RDX as usize] as u64;
            prefix.registers[UNW_X86_64_RCX] =
                ucontext.uc_mcontext.gregs[libc::REG_RCX as usize] as u64;
            prefix.registers[UNW_X86_64_RBX] =
                ucontext.uc_mcontext.gregs[libc::REG_RBX as usize] as u64;
            prefix.registers[UNW_X86_64_RSI] =
                ucontext.uc_mcontext.gregs[libc::REG_RSI as usize] as u64;
            prefix.registers[UNW_X86_64_RDI] =
                ucontext.uc_mcontext.gregs[libc::REG_RDI as usize] as u64;
            prefix.registers[UNW_X86_64_RBP] =
                ucontext.uc_mcontext.gregs[libc::REG_RBP as usize] as u64;
            prefix.registers[UNW_X86_64_RSP] =
                ucontext.uc_mcontext.gregs[libc::REG_RSP as usize] as u64;
            prefix.registers[UNW_X86_64_R8] =
                ucontext.uc_mcontext.gregs[libc::REG_R8 as usize] as u64;
            prefix.registers[UNW_X86_64_R9] =
                ucontext.uc_mcontext.gregs[libc::REG_R9 as usize] as u64;
            prefix.registers[UNW_X86_64_R10] =
                ucontext.uc_mcontext.gregs[libc::REG_R10 as usize] as u64;
            prefix.registers[UNW_X86_64_R11] =
                ucontext.uc_mcontext.gregs[libc::REG_R11 as usize] as u64;
            prefix.registers[UNW_X86_64_R12] =
                ucontext.uc_mcontext.gregs[libc::REG_R12 as usize] as u64;
            prefix.registers[UNW_X86_64_R13] =
                ucontext.uc_mcontext.gregs[libc::REG_R13 as usize] as u64;
            prefix.registers[UNW_X86_64_R14] =
                ucontext.uc_mcontext.gregs[libc::REG_R14 as usize] as u64;
            prefix.registers[UNW_X86_64_R15] =
                ucontext.uc_mcontext.gregs[libc::REG_R15 as usize] as u64;
            prefix.registers[UNW_X86_64_RIP] =
                ucontext.uc_mcontext.gregs[libc::REG_RIP as usize] as u64;
            prefix.stack_base = prefix.registers[UNW_X86_64_RSP];

            let vectors = [
                libc::iovec {
                    iov_base: ptr::from_mut(&mut prefix).cast::<c_void>(),
                    iov_len: size_of::<SnapshotPrefix>(),
                },
                libc::iovec {
                    iov_base: prefix.stack_base as usize as *mut c_void,
                    iov_len: SNAPSHOT_STACK_SIZE,
                },
            ];
            let written = unsafe { libc::writev(fd, vectors.as_ptr(), vectors.len() as c_int) };
            if written < 0 {
                error = unsafe { *libc::__errno_location() };
            } else if written as usize != size_of::<Snapshot>() {
                error = libc::EIO;
            }
        }

        if error != 0 {
            HANDLER_ERROR.store(error, Ordering::Release);
        }
        HANDLER_ACTIVE.store(0, Ordering::Release);
    }

    pub fn capture_raw_frames(pid: i32, tid: i32, timeout_ms: i32) -> Result<Vec<*mut c_void>> {
        let _guard = BACKTRACE_MUTEX
            .try_lock()
            .map_err(|error| anyhow!("Failed to acquire backtrace lock: {error}"))?;

        init_pipe()?;
        drain_pipe()?;
        HANDLER_ERROR.store(0, Ordering::Release);

        let send_result = unsafe { libc::syscall(libc::SYS_tgkill, pid, tid, libc::SIGUSR2) };
        if send_result != 0 {
            return Err(anyhow!(
                "Failed to send SIGUSR2 to process {pid} (thread: {tid}): {}",
                io::Error::last_os_error()
            ));
        }

        let snapshot = match read_snapshot(timeout_ms) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                let _ = drain_pipe();
                return Err(error);
            }
        };
        unwind_snapshot(&snapshot)
    }

    fn init_pipe() -> Result<()> {
        let mut initialization_error = None;
        PIPE_INIT.call_once(|| {
            let mut fds = [-1; 2];
            let result =
                unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) };
            if result != 0 {
                initialization_error = Some(io::Error::last_os_error());
                return;
            }

            PIPE_READ_FD.store(fds[0], Ordering::Release);
            PIPE_WRITE_FD.store(fds[1], Ordering::Release);
        });

        if let Some(error) = initialization_error {
            return Err(anyhow!("Failed to create stack snapshot pipe: {error}"));
        }
        if PIPE_READ_FD.load(Ordering::Acquire) < 0 {
            return Err(anyhow!("Stack snapshot pipe initialization failed"));
        }
        Ok(())
    }

    fn drain_pipe() -> Result<()> {
        let fd = PIPE_READ_FD.load(Ordering::Acquire);
        if fd < 0 {
            return Ok(());
        }

        let mut buffer = [0_u8; 512];
        loop {
            let read =
                unsafe { libc::read(fd, buffer.as_mut_ptr().cast::<c_void>(), buffer.len()) };
            if read > 0 {
                continue;
            }
            if read == 0 {
                return Err(anyhow!("Unexpected EOF while draining snapshot pipe"));
            }

            let error = io::Error::last_os_error();
            match error.raw_os_error() {
                Some(libc::EAGAIN) => return Ok(()),
                Some(libc::EINTR) => continue,
                _ => return Err(anyhow!("Failed to drain snapshot pipe: {error}")),
            }
        }
    }

    fn read_snapshot(timeout_ms: i32) -> Result<Box<Snapshot>> {
        let fd = PIPE_READ_FD.load(Ordering::Acquire);
        if fd < 0 {
            return Err(anyhow!("Stack snapshot pipe is not initialized"));
        }

        let mut snapshot = Box::new(Snapshot::zeroed());
        let snapshot_size = size_of::<Snapshot>();
        let snapshot_bytes = unsafe {
            std::slice::from_raw_parts_mut(
                ptr::from_mut(snapshot.as_mut()).cast::<u8>(),
                snapshot_size,
            )
        };
        let mut bytes_read = 0;
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(timeout_ms.max(0) as u64))
            .ok_or_else(|| anyhow!("Invalid stack snapshot timeout"))?;

        while bytes_read < snapshot_size {
            let handler_error = HANDLER_ERROR.load(Ordering::Acquire);
            if handler_error != 0 {
                return Err(anyhow!(
                    "SIGUSR2 snapshot handler failed: {}",
                    io::Error::from_raw_os_error(handler_error)
                ));
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(anyhow!(
                    "Timeout reading stack snapshot ({bytes_read} of {snapshot_size} bytes)"
                ));
            }
            let remaining_ms = deadline
                .saturating_duration_since(now)
                .as_millis()
                .min(POLL_INTERVAL_MS as u128) as i32;
            let mut poll_fd = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let poll_result = unsafe { libc::poll(&mut poll_fd, 1, remaining_ms.max(1)) };
            if poll_result < 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                return Err(anyhow!("Failed to poll stack snapshot pipe: {error}"));
            }
            if poll_result == 0 {
                continue;
            }
            if poll_fd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                return Err(anyhow!(
                    "Stack snapshot pipe reported events 0x{:x}",
                    poll_fd.revents
                ));
            }

            let read = unsafe {
                libc::read(
                    fd,
                    snapshot_bytes[bytes_read..].as_mut_ptr().cast::<c_void>(),
                    snapshot_size - bytes_read,
                )
            };
            if read > 0 {
                bytes_read += read as usize;
                continue;
            }
            if read == 0 {
                return Err(anyhow!("Unexpected EOF while reading stack snapshot"));
            }

            let error = io::Error::last_os_error();
            match error.raw_os_error() {
                Some(libc::EAGAIN) | Some(libc::EINTR) => continue,
                _ => return Err(anyhow!("Failed to read stack snapshot: {error}")),
            }
        }

        let handler_error = HANDLER_ERROR.load(Ordering::Acquire);
        if handler_error != 0 {
            return Err(anyhow!(
                "SIGUSR2 snapshot handler failed: {}",
                io::Error::from_raw_os_error(handler_error)
            ));
        }
        if !snapshot.is_valid() {
            return Err(anyhow!("Invalid stack snapshot record"));
        }
        Ok(snapshot)
    }

    unsafe fn unwind_context<'a>(argument: *mut c_void) -> &'a UnwindContext {
        unsafe { &*argument.cast::<UnwindContext>() }
    }

    unsafe fn local_address_space() -> UnwAddrSpace {
        unsafe { UNW_LOCAL_ADDR_SPACE }
    }

    unsafe extern "C" fn snapshot_find_proc_info(
        _address_space: UnwAddrSpace,
        ip: UnwWord,
        proc_info: *mut c_void,
        need_unwind_info: c_int,
        argument: *mut c_void,
    ) -> c_int {
        let context = unsafe { unwind_context(argument) };
        let accessors = unsafe { &*context.local_accessors };
        match accessors.find_proc_info {
            Some(callback) => unsafe {
                callback(
                    local_address_space(),
                    ip,
                    proc_info,
                    need_unwind_info,
                    ptr::null_mut(),
                )
            },
            None => -UNW_EUNSPEC,
        }
    }

    unsafe extern "C" fn snapshot_put_unwind_info(
        _address_space: UnwAddrSpace,
        proc_info: *mut c_void,
        argument: *mut c_void,
    ) {
        let context = unsafe { unwind_context(argument) };
        let accessors = unsafe { &*context.local_accessors };
        if let Some(callback) = accessors.put_unwind_info {
            unsafe { callback(local_address_space(), proc_info, ptr::null_mut()) };
        }
    }

    unsafe extern "C" fn snapshot_get_dyn_info_list_addr(
        _address_space: UnwAddrSpace,
        address: *mut UnwWord,
        argument: *mut c_void,
    ) -> c_int {
        let context = unsafe { unwind_context(argument) };
        let accessors = unsafe { &*context.local_accessors };
        match accessors.get_dyn_info_list_addr {
            Some(callback) => unsafe { callback(local_address_space(), address, ptr::null_mut()) },
            None => -UNW_EUNSPEC,
        }
    }

    unsafe extern "C" fn snapshot_access_mem(
        _address_space: UnwAddrSpace,
        address: UnwWord,
        value: *mut UnwWord,
        write: c_int,
        argument: *mut c_void,
    ) -> c_int {
        if write != 0 {
            return -UNW_EREADONLYREG;
        }

        let context = unsafe { unwind_context(argument) };
        let snapshot = unsafe { &*context.snapshot };
        let stack_base = snapshot.prefix.stack_base;
        if address >= stack_base {
            let offset = (address - stack_base) as usize;
            if offset <= SNAPSHOT_STACK_SIZE - size_of::<UnwWord>() {
                unsafe {
                    ptr::copy_nonoverlapping(
                        snapshot.stack.as_ptr().add(offset),
                        value.cast::<u8>(),
                        size_of::<UnwWord>(),
                    );
                }
                return 0;
            }
            return -UNW_EINVAL;
        }

        let accessors = unsafe { &*context.local_accessors };
        match accessors.access_mem {
            Some(callback) => unsafe {
                callback(local_address_space(), address, value, 0, ptr::null_mut())
            },
            None => -UNW_EUNSPEC,
        }
    }

    unsafe extern "C" fn snapshot_access_reg(
        _address_space: UnwAddrSpace,
        register: c_int,
        value: *mut UnwWord,
        write: c_int,
        argument: *mut c_void,
    ) -> c_int {
        if write != 0 {
            return -UNW_EREADONLYREG;
        }
        if register < 0 || register as usize >= SNAPSHOT_REGISTER_COUNT {
            return -UNW_EBADREG;
        }

        let context = unsafe { unwind_context(argument) };
        let snapshot = unsafe { &*context.snapshot };
        unsafe {
            *value = snapshot.prefix.registers[register as usize];
        }
        0
    }

    unsafe extern "C" fn snapshot_access_fpreg(
        _address_space: UnwAddrSpace,
        _register: c_int,
        _value: *mut c_void,
        _write: c_int,
        _argument: *mut c_void,
    ) -> c_int {
        -UNW_EBADREG
    }

    unsafe extern "C" fn snapshot_resume(
        _address_space: UnwAddrSpace,
        _cursor: *mut UnwCursor,
        _argument: *mut c_void,
    ) -> c_int {
        -UNW_EINVAL
    }

    unsafe extern "C" fn snapshot_get_proc_name(
        _address_space: UnwAddrSpace,
        ip: UnwWord,
        buffer: *mut c_char,
        length: usize,
        offset: *mut UnwWord,
        argument: *mut c_void,
    ) -> c_int {
        let context = unsafe { unwind_context(argument) };
        let accessors = unsafe { &*context.local_accessors };
        match accessors.get_proc_name {
            Some(callback) => unsafe {
                callback(
                    local_address_space(),
                    ip,
                    buffer,
                    length,
                    offset,
                    ptr::null_mut(),
                )
            },
            None => -UNW_EUNSPEC,
        }
    }

    fn unwind_snapshot(snapshot: &Snapshot) -> Result<Vec<*mut c_void>> {
        if !snapshot.is_valid() {
            return Err(anyhow!("Invalid stack snapshot record"));
        }

        let local_address_space = unsafe { local_address_space() };
        let local_accessors = unsafe { unw_get_accessors(local_address_space) };
        if local_accessors.is_null() {
            return Err(anyhow!("libunwind local accessors are unavailable"));
        }

        let mut accessors = UnwAccessors {
            find_proc_info: Some(snapshot_find_proc_info),
            put_unwind_info: Some(snapshot_put_unwind_info),
            get_dyn_info_list_addr: Some(snapshot_get_dyn_info_list_addr),
            access_mem: Some(snapshot_access_mem),
            access_reg: Some(snapshot_access_reg),
            access_fpreg: Some(snapshot_access_fpreg),
            resume: Some(snapshot_resume),
            get_proc_name: Some(snapshot_get_proc_name),
        };
        let address_space = unsafe { unw_create_addr_space(&mut accessors, 0) };
        if address_space.is_null() {
            return Err(anyhow!(
                "libunwind failed to create address space ({})",
                -UNW_ENOMEM
            ));
        }
        unsafe {
            unw_set_caching_policy(address_space, UNW_CACHE_NONE);
        }

        let mut context = UnwindContext {
            snapshot,
            local_accessors,
        };
        let mut cursor = UnwCursor { opaque: [0; 127] };
        let init_result = unsafe {
            unw_init_remote(
                &mut cursor,
                address_space,
                ptr::from_mut(&mut context).cast::<c_void>(),
            )
        };
        if init_result < 0 {
            unsafe { unw_destroy_addr_space(address_space) };
            return Err(anyhow!(
                "libunwind failed to initialize snapshot cursor (error {init_result})"
            ));
        }

        let mut ips = Vec::with_capacity(MAX_FRAMES);
        while ips.len() < MAX_FRAMES {
            let mut ip = 0;
            let get_reg_result = unsafe { unw_get_reg(&mut cursor, UNW_REG_IP, &mut ip) };
            if get_reg_result < 0 {
                if ips.is_empty() {
                    unsafe { unw_destroy_addr_space(address_space) };
                    return Err(anyhow!(
                        "libunwind failed to read instruction pointer (error {get_reg_result})"
                    ));
                }
                break;
            }
            if ip == 0 {
                break;
            }
            ips.push(ip as usize as *mut c_void);

            if unsafe { unw_step(&mut cursor) } <= 0 {
                break;
            }
        }

        unsafe { unw_destroy_addr_space(address_space) };
        Ok(ips)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn snapshot_record_fits_in_pipe_buf() {
            assert!(size_of::<Snapshot>() > SNAPSHOT_STACK_SIZE);
            assert!(size_of::<Snapshot>() < 4096);
        }

        #[test]
        fn captures_current_thread_snapshot() {
            let pid = unsafe { libc::getpid() };
            let tid = unsafe { libc::syscall(libc::SYS_gettid) as i32 };
            for _ in 0..3 {
                let frames = capture_raw_frames(pid, tid, 2000).unwrap();
                assert!(!frames.is_empty());
            }
        }

        #[test]
        fn rejects_invalid_snapshot_record() {
            let invalid = Snapshot::zeroed();
            let error = unwind_snapshot(&invalid).unwrap_err();
            assert!(error.to_string().contains("Invalid stack snapshot"));
        }
    }
}

#[cfg(probing_snapshot_unwind)]
pub use supported::{backtrace_signal_handler, capture_raw_frames};

#[cfg(not(probing_snapshot_unwind))]
pub fn capture_raw_frames(
    _pid: i32,
    _tid: i32,
    _timeout_ms: i32,
) -> Result<Vec<*mut libc::c_void>> {
    anyhow::bail!("SIGUSR2 stack snapshots require Linux x86_64")
}
