#[cfg(not(probing_snapshot_unwind))]
use anyhow::Result;
#[cfg(not(probing_snapshot_unwind))]
use nix::libc;

#[cfg(probing_snapshot_unwind)]
mod supported {
    use std::fs;
    use std::io;
    use std::mem::size_of;
    use std::os::raw::{c_char, c_int, c_void};
    use std::ptr;
    use std::sync::atomic::{AtomicI32, AtomicPtr, AtomicU64, AtomicUsize, Ordering};
    use std::sync::Once;
    use std::time::{Duration, Instant};

    use anyhow::{anyhow, Result};
    #[cfg(not(probing_bundled_libunwind))]
    use libloading::Library;
    use nix::libc;
    use once_cell::sync::Lazy;

    const MAX_FRAMES: usize = 512;
    const POLL_INTERVAL_MS: i32 = 20;
    const SNAPSHOT_MAGIC: u32 = 0x5052_4254;
    const SNAPSHOT_VERSION: u16 = 2;
    const SNAPSHOT_REGISTER_COUNT: usize = 17;
    const MAX_STACK_SNAPSHOT_SIZE: usize = 64 * 1024 * 1024;
    const CAPTURE_IDLE: i32 = 0;
    const CAPTURE_CONTEXT: i32 = 1;
    const CAPTURE_STACK: i32 = 2;

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
    static STACK_BUFFER_INIT: Once = Once::new();
    static HANDLER_ERROR: AtomicI32 = AtomicI32::new(0);
    static HANDLER_ACTIVE: AtomicI32 = AtomicI32::new(0);
    static CAPTURE_MODE: AtomicI32 = AtomicI32::new(CAPTURE_IDLE);
    static STACK_LOWER_BOUND: AtomicU64 = AtomicU64::new(0);
    static STACK_UPPER_BOUND: AtomicU64 = AtomicU64::new(0);
    static STACK_BUFFER: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
    static STACK_BUFFER_CAPACITY: AtomicUsize = AtomicUsize::new(0);
    static SNAPSHOT_READY: AtomicI32 = AtomicI32::new(0);

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct SnapshotPrefix {
        magic: u32,
        version: u16,
        register_count: u16,
        stack_size: u32,
        reserved: u32,
        stack_base: u64,
        registers: [u64; SNAPSHOT_REGISTER_COUNT],
    }

    struct Snapshot<'a> {
        prefix: SnapshotPrefix,
        stack: &'a [u8],
        mapping_start: u64,
        mapping_end: u64,
    }

    impl Snapshot<'_> {
        fn is_valid(&self) -> bool {
            self.prefix.magic == SNAPSHOT_MAGIC
                && self.prefix.version == SNAPSHOT_VERSION
                && self.prefix.register_count as usize == SNAPSHOT_REGISTER_COUNT
                && self.prefix.stack_size as usize == self.stack.len()
                && !self.stack.is_empty()
                && self.prefix.stack_base == self.prefix.registers[UNW_X86_64_RSP]
                && self.mapping_start <= self.prefix.stack_base
                && self.prefix.stack_base.checked_add(self.stack.len() as u64)
                    == Some(self.mapping_end)
        }
    }

    const _: () = assert!(size_of::<SnapshotPrefix>() < 4096);

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

    struct UnwindContext<'a> {
        snapshot: *const Snapshot<'a>,
        local_address_space: UnwAddrSpace,
        local_accessors: *const UnwAccessors,
    }

    type CreateAddrSpace = unsafe extern "C" fn(*mut UnwAccessors, c_int) -> UnwAddrSpace;
    type DestroyAddrSpace = unsafe extern "C" fn(UnwAddrSpace);
    type GetAccessors = unsafe extern "C" fn(UnwAddrSpace) -> *mut UnwAccessors;
    type SetCachingPolicy = unsafe extern "C" fn(UnwAddrSpace, c_int) -> c_int;
    type InitRemote = unsafe extern "C" fn(*mut UnwCursor, UnwAddrSpace, *mut c_void) -> c_int;
    type GetReg = unsafe extern "C" fn(*mut UnwCursor, c_int, *mut UnwWord) -> c_int;
    type Step = unsafe extern "C" fn(*mut UnwCursor) -> c_int;

    #[cfg(probing_bundled_libunwind)]
    extern "C" {
        static _Ux86_64_local_addr_space: UnwAddrSpace;
        fn _Ux86_64_create_addr_space(
            accessors: *mut UnwAccessors,
            byte_order: c_int,
        ) -> UnwAddrSpace;
        fn _Ux86_64_destroy_addr_space(address_space: UnwAddrSpace);
        fn _Ux86_64_get_accessors(address_space: UnwAddrSpace) -> *mut UnwAccessors;
        fn _Ux86_64_set_caching_policy(address_space: UnwAddrSpace, policy: c_int) -> c_int;
        fn _Ux86_64_init_remote(
            cursor: *mut UnwCursor,
            address_space: UnwAddrSpace,
            argument: *mut c_void,
        ) -> c_int;
        fn _Ux86_64_get_reg(cursor: *mut UnwCursor, register: c_int, value: *mut UnwWord) -> c_int;
        fn _Ux86_64_step(cursor: *mut UnwCursor) -> c_int;
    }

    struct Libunwind {
        #[cfg(not(probing_bundled_libunwind))]
        _library: Library,
        local_address_space: usize,
        create_addr_space: CreateAddrSpace,
        destroy_addr_space: DestroyAddrSpace,
        get_accessors: GetAccessors,
        set_caching_policy: SetCachingPolicy,
        init_remote: InitRemote,
        get_reg: GetReg,
        step: Step,
    }

    impl Libunwind {
        #[cfg(probing_bundled_libunwind)]
        fn load() -> std::result::Result<Self, String> {
            let local_address_space = unsafe { _Ux86_64_local_addr_space };
            if local_address_space.is_null() {
                return Err("_Ux86_64_local_addr_space symbol is null".to_string());
            }

            Ok(Self {
                local_address_space: local_address_space as usize,
                create_addr_space: _Ux86_64_create_addr_space,
                destroy_addr_space: _Ux86_64_destroy_addr_space,
                get_accessors: _Ux86_64_get_accessors,
                set_caching_policy: _Ux86_64_set_caching_policy,
                init_remote: _Ux86_64_init_remote,
                get_reg: _Ux86_64_get_reg,
                step: _Ux86_64_step,
            })
        }

        #[cfg(not(probing_bundled_libunwind))]
        fn load() -> std::result::Result<Self, String> {
            // Loading happens outside the signal handler, on the first native
            // stack request. Constructors in the target library may run here.
            let library = match unsafe { Library::new("libunwind-x86_64.so.8") } {
                Ok(library) => library,
                Err(versioned_error) => {
                    unsafe { Library::new("libunwind-x86_64.so") }.map_err(|unversioned_error| {
                        format!(
                            "libunwind is unavailable: libunwind-x86_64.so.8: \
                                 {versioned_error}; libunwind-x86_64.so: {unversioned_error}"
                        )
                    })?
                }
            };

            let local_address_space = {
                // This symbol is a global variable of type unw_addr_space_t,
                // so dlsym returns a pointer to the variable.
                let symbol =
                    unsafe { library.get::<*mut UnwAddrSpace>(b"_Ux86_64_local_addr_space\0") }
                        .map_err(|error| {
                            format!("failed to load _Ux86_64_local_addr_space: {error}")
                        })?;
                let symbol_address = *symbol;
                if symbol_address.is_null() {
                    return Err("_Ux86_64_local_addr_space symbol is null".to_string());
                }
                (unsafe { *symbol_address }) as usize
            };

            Ok(Self {
                create_addr_space: load_symbol(&library, b"_Ux86_64_create_addr_space\0")?,
                destroy_addr_space: load_symbol(&library, b"_Ux86_64_destroy_addr_space\0")?,
                get_accessors: load_symbol(&library, b"_Ux86_64_get_accessors\0")?,
                set_caching_policy: load_symbol(&library, b"_Ux86_64_set_caching_policy\0")?,
                init_remote: load_symbol(&library, b"_Ux86_64_init_remote\0")?,
                get_reg: load_symbol(&library, b"_Ux86_64_get_reg\0")?,
                step: load_symbol(&library, b"_Ux86_64_step\0")?,
                _library: library,
                local_address_space,
            })
        }

        fn local_address_space(&self) -> UnwAddrSpace {
            self.local_address_space as UnwAddrSpace
        }
    }

    #[cfg(not(probing_bundled_libunwind))]
    fn load_symbol<T: Copy>(library: &Library, name: &[u8]) -> std::result::Result<T, String> {
        // The caller supplies the exact libunwind ABI type for each symbol.
        unsafe { library.get::<T>(name) }
            .map(|symbol| *symbol)
            .map_err(|error| {
                let name = String::from_utf8_lossy(name);
                format!(
                    "failed to load libunwind symbol {}: {error}",
                    name.trim_end_matches('\0')
                )
            })
    }

    static LIBUNWIND: Lazy<std::result::Result<Libunwind, String>> = Lazy::new(Libunwind::load);

    fn libunwind() -> Result<&'static Libunwind> {
        match &*LIBUNWIND {
            Ok(libunwind) => Ok(libunwind),
            Err(error) => Err(anyhow!("{error}")),
        }
    }

    /// The SIGUSR2 handler only copies registers and, for the second phase,
    /// bytes from the interrupted stack into a buffer allocated beforehand.
    /// It performs no formatting, allocation, logging, locking, or unwind.
    pub extern "C" fn backtrace_signal_handler(
        signal: c_int,
        info: *mut libc::siginfo_t,
        context: *mut c_void,
    ) {
        let _ = signal;
        let _ = info;
        // POSIX permits an async-signal-safe handler to inspect errno only if
        // it preserves the interrupted thread's value across the handler.
        let errno_location = unsafe { libc::__errno_location() };
        let saved_errno = unsafe { *errno_location };

        if HANDLER_ACTIVE
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            HANDLER_ERROR.store(libc::EBUSY, Ordering::Release);
            unsafe {
                *errno_location = saved_errno;
            }
            return;
        }

        let error: c_int;
        let fd = PIPE_WRITE_FD.load(Ordering::Acquire);
        if fd < 0 || context.is_null() {
            error = if fd < 0 { libc::EBADF } else { libc::EINVAL };
        } else {
            let ucontext = unsafe { &*context.cast::<libc::ucontext_t>() };
            let mut prefix = snapshot_prefix(ucontext);

            match CAPTURE_MODE.load(Ordering::Acquire) {
                CAPTURE_CONTEXT => {
                    error = unsafe { write_prefix(fd, &prefix) };
                }
                CAPTURE_STACK => {
                    let stack_base = prefix.stack_base;
                    let lower_bound = STACK_LOWER_BOUND.load(Ordering::Acquire);
                    let upper_bound = STACK_UPPER_BOUND.load(Ordering::Acquire);
                    let destination = STACK_BUFFER.load(Ordering::Acquire);
                    let capacity = STACK_BUFFER_CAPACITY.load(Ordering::Acquire);

                    if destination.is_null()
                        || stack_base < lower_bound
                        || stack_base >= upper_bound
                    {
                        error = libc::EFAULT;
                    } else {
                        let stack_size = (upper_bound - stack_base) as usize;
                        if stack_size == 0
                            || stack_size > capacity
                            || stack_size > u32::MAX as usize
                        {
                            error = libc::EOVERFLOW;
                        } else {
                            unsafe {
                                copy_stack_bytes(
                                    stack_base as usize as *const u8,
                                    destination,
                                    stack_size,
                                );
                            }
                            prefix.stack_size = stack_size as u32;
                            SNAPSHOT_READY.store(1, Ordering::Release);
                            error = unsafe { write_prefix(fd, &prefix) };
                        }
                    }
                }
                _ => {
                    error = libc::EINVAL;
                }
            }
        }

        if error != 0 {
            HANDLER_ERROR.store(error, Ordering::Release);
        }
        HANDLER_ACTIVE.store(0, Ordering::Release);
        unsafe {
            *errno_location = saved_errno;
        }
    }

    fn snapshot_prefix(ucontext: &libc::ucontext_t) -> SnapshotPrefix {
        let mut prefix = SnapshotPrefix {
            magic: SNAPSHOT_MAGIC,
            version: SNAPSHOT_VERSION,
            register_count: SNAPSHOT_REGISTER_COUNT as u16,
            stack_size: 0,
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
        prefix.registers[UNW_X86_64_R8] = ucontext.uc_mcontext.gregs[libc::REG_R8 as usize] as u64;
        prefix.registers[UNW_X86_64_R9] = ucontext.uc_mcontext.gregs[libc::REG_R9 as usize] as u64;
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
        prefix
    }

    unsafe fn write_prefix(fd: c_int, prefix: &SnapshotPrefix) -> c_int {
        let written = unsafe {
            libc::write(
                fd,
                ptr::from_ref(prefix).cast::<c_void>(),
                size_of::<SnapshotPrefix>(),
            )
        };
        if written < 0 {
            unsafe { *libc::__errno_location() }
        } else if written as usize != size_of::<SnapshotPrefix>() {
            libc::EIO
        } else {
            0
        }
    }

    #[inline(always)]
    unsafe fn copy_stack_bytes(source: *const u8, destination: *mut u8, length: usize) {
        unsafe {
            core::arch::asm!(
                "cld",
                "rep movsb",
                inout("rcx") length => _,
                inout("rsi") source => _,
                inout("rdi") destination => _,
                options(nostack)
            );
        }
    }

    struct CaptureReset;

    impl Drop for CaptureReset {
        fn drop(&mut self) {
            CAPTURE_MODE.store(CAPTURE_IDLE, Ordering::Release);
            STACK_LOWER_BOUND.store(0, Ordering::Release);
            STACK_UPPER_BOUND.store(0, Ordering::Release);
        }
    }

    pub fn capture_raw_frames(pid: i32, tid: i32, timeout_ms: i32) -> Result<Vec<*mut c_void>> {
        let _guard = BACKTRACE_MUTEX
            .try_lock()
            .map_err(|error| anyhow!("Failed to acquire backtrace lock: {error}"))?;
        let _capture_reset = CaptureReset;

        init_pipe()?;
        init_stack_buffer()?;
        drain_pipe()?;

        let context = capture_prefix(pid, tid, CAPTURE_CONTEXT, timeout_ms)?;
        let stack_pointer = context.registers[UNW_X86_64_RSP];
        let (mapping_start, mapping_end) = find_stack_mapping(pid, tid, stack_pointer)?;
        let capture_start =
            mapping_start.max(mapping_end.saturating_sub(MAX_STACK_SNAPSHOT_SIZE as u64));
        if stack_pointer < capture_start {
            return Err(anyhow!(
                "Thread stack requires more than {} bytes to snapshot",
                MAX_STACK_SNAPSHOT_SIZE
            ));
        }

        STACK_LOWER_BOUND.store(capture_start, Ordering::Release);
        STACK_UPPER_BOUND.store(mapping_end, Ordering::Release);
        SNAPSHOT_READY.store(0, Ordering::Release);

        let prefix = capture_prefix(pid, tid, CAPTURE_STACK, timeout_ms)?;
        if SNAPSHOT_READY.load(Ordering::Acquire) != 1 {
            return Err(anyhow!("SIGUSR2 stack snapshot was not published"));
        }

        let stack_size = prefix.stack_size as usize;
        let capacity = STACK_BUFFER_CAPACITY.load(Ordering::Acquire);
        let buffer = STACK_BUFFER.load(Ordering::Acquire);
        if buffer.is_null() || stack_size == 0 || stack_size > capacity {
            return Err(anyhow!("Invalid stack snapshot size {stack_size}"));
        }
        let stack = unsafe { std::slice::from_raw_parts(buffer.cast_const(), stack_size) };
        let snapshot = Snapshot {
            prefix,
            stack,
            mapping_start,
            mapping_end,
        };
        unwind_snapshot(&snapshot)
    }

    fn capture_prefix(pid: i32, tid: i32, mode: i32, timeout_ms: i32) -> Result<SnapshotPrefix> {
        HANDLER_ERROR.store(0, Ordering::Release);
        CAPTURE_MODE.store(mode, Ordering::Release);

        let send_result = unsafe { libc::syscall(libc::SYS_tgkill, pid, tid, libc::SIGUSR2) };
        if send_result != 0 {
            return Err(anyhow!(
                "Failed to send SIGUSR2 to process {pid} (thread: {tid}): {}",
                io::Error::last_os_error()
            ));
        }

        read_prefix(timeout_ms).inspect_err(|_| {
            let _ = drain_pipe();
        })
    }

    fn find_stack_mapping(pid: i32, tid: i32, stack_pointer: u64) -> Result<(u64, u64)> {
        let maps_path = format!("/proc/{pid}/task/{tid}/maps");
        let maps = fs::read_to_string(&maps_path)
            .map_err(|error| anyhow!("Failed to read {maps_path}: {error}"))?;

        for line in maps.lines() {
            let mut fields = line.split_whitespace();
            let Some(range) = fields.next() else {
                continue;
            };
            let Some(permissions) = fields.next() else {
                continue;
            };
            if !permissions.starts_with('r') {
                continue;
            }
            let Some((start, end)) = range.split_once('-') else {
                continue;
            };
            let Ok(start) = u64::from_str_radix(start, 16) else {
                continue;
            };
            let Ok(end) = u64::from_str_radix(end, 16) else {
                continue;
            };
            if stack_pointer >= start && stack_pointer < end {
                return Ok((start, end));
            }
        }

        Err(anyhow!(
            "Unable to find a readable stack mapping containing RSP 0x{stack_pointer:x}"
        ))
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

    fn init_stack_buffer() -> Result<()> {
        let mut initialization_error = None;
        STACK_BUFFER_INIT.call_once(|| {
            let buffer = unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    MAX_STACK_SNAPSHOT_SIZE,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
            if buffer == libc::MAP_FAILED {
                initialization_error = Some(io::Error::last_os_error());
                return;
            }
            STACK_BUFFER.store(buffer.cast::<u8>(), Ordering::Release);
            STACK_BUFFER_CAPACITY.store(MAX_STACK_SNAPSHOT_SIZE, Ordering::Release);
        });

        if let Some(error) = initialization_error {
            return Err(anyhow!("Failed to allocate stack snapshot buffer: {error}"));
        }
        if STACK_BUFFER.load(Ordering::Acquire).is_null() {
            return Err(anyhow!("Stack snapshot buffer initialization failed"));
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

    fn read_prefix(timeout_ms: i32) -> Result<SnapshotPrefix> {
        let fd = PIPE_READ_FD.load(Ordering::Acquire);
        if fd < 0 {
            return Err(anyhow!("Stack snapshot pipe is not initialized"));
        }

        let mut prefix = SnapshotPrefix {
            magic: 0,
            version: 0,
            register_count: 0,
            stack_size: 0,
            reserved: 0,
            stack_base: 0,
            registers: [0; SNAPSHOT_REGISTER_COUNT],
        };
        let prefix_bytes = unsafe {
            std::slice::from_raw_parts_mut(
                ptr::from_mut(&mut prefix).cast::<u8>(),
                size_of::<SnapshotPrefix>(),
            )
        };
        let mut bytes_read = 0;
        let deadline = Instant::now()
            .checked_add(Duration::from_millis(timeout_ms.max(0) as u64))
            .ok_or_else(|| anyhow!("Invalid stack snapshot timeout"))?;

        while bytes_read < prefix_bytes.len() {
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
                    "Timeout reading stack snapshot metadata ({bytes_read} of {} bytes)",
                    prefix_bytes.len()
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
                    prefix_bytes[bytes_read..].as_mut_ptr().cast::<c_void>(),
                    prefix_bytes.len() - bytes_read,
                )
            };
            if read > 0 {
                bytes_read += read as usize;
                continue;
            }
            if read == 0 {
                return Err(anyhow!(
                    "Unexpected EOF while reading stack snapshot metadata"
                ));
            }

            let error = io::Error::last_os_error();
            match error.raw_os_error() {
                Some(libc::EAGAIN) | Some(libc::EINTR) => continue,
                _ => return Err(anyhow!("Failed to read stack snapshot metadata: {error}")),
            }
        }

        while HANDLER_ACTIVE.load(Ordering::Acquire) != 0 {
            if Instant::now() >= deadline {
                return Err(anyhow!("Timeout waiting for SIGUSR2 snapshot handler"));
            }
            std::thread::yield_now();
        }

        let handler_error = HANDLER_ERROR.load(Ordering::Acquire);
        if handler_error != 0 {
            return Err(anyhow!(
                "SIGUSR2 snapshot handler failed: {}",
                io::Error::from_raw_os_error(handler_error)
            ));
        }
        if prefix.magic != SNAPSHOT_MAGIC
            || prefix.version != SNAPSHOT_VERSION
            || prefix.register_count as usize != SNAPSHOT_REGISTER_COUNT
        {
            return Err(anyhow!("Invalid stack snapshot metadata"));
        }
        Ok(prefix)
    }

    unsafe fn unwind_context<'a>(argument: *mut c_void) -> &'a UnwindContext<'a> {
        unsafe { &*argument.cast::<UnwindContext>() }
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
                    context.local_address_space,
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
            unsafe { callback(context.local_address_space, proc_info, ptr::null_mut()) };
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
            Some(callback) => unsafe {
                callback(context.local_address_space, address, ptr::null_mut())
            },
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
        let word_size = size_of::<UnwWord>() as u64;
        let word_end = address.checked_add(word_size);
        if address >= stack_base && matches!(word_end, Some(end) if end <= snapshot.mapping_end) {
            let offset = (address - stack_base) as usize;
            unsafe {
                ptr::copy_nonoverlapping(
                    snapshot.stack.as_ptr().add(offset),
                    value.cast::<u8>(),
                    size_of::<UnwWord>(),
                );
            }
            return 0;
        }
        if address >= snapshot.mapping_start && address < snapshot.mapping_end {
            return -UNW_EINVAL;
        }

        let accessors = unsafe { &*context.local_accessors };
        match accessors.access_mem {
            Some(callback) => unsafe {
                callback(
                    context.local_address_space,
                    address,
                    value,
                    0,
                    ptr::null_mut(),
                )
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
                    context.local_address_space,
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

        let libunwind = libunwind()?;
        let local_address_space = libunwind.local_address_space();
        let local_accessors = unsafe { (libunwind.get_accessors)(local_address_space) };
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
        let address_space = unsafe { (libunwind.create_addr_space)(&mut accessors, 0) };
        if address_space.is_null() {
            return Err(anyhow!(
                "libunwind failed to create address space ({})",
                -UNW_ENOMEM
            ));
        }
        unsafe {
            (libunwind.set_caching_policy)(address_space, UNW_CACHE_NONE);
        }

        let mut context = UnwindContext {
            snapshot,
            local_address_space,
            local_accessors,
        };
        let mut cursor = UnwCursor { opaque: [0; 127] };
        let init_result = unsafe {
            (libunwind.init_remote)(
                &mut cursor,
                address_space,
                ptr::from_mut(&mut context).cast::<c_void>(),
            )
        };
        if init_result < 0 {
            unsafe { (libunwind.destroy_addr_space)(address_space) };
            return Err(anyhow!(
                "libunwind failed to initialize snapshot cursor (error {init_result})"
            ));
        }

        let mut ips = Vec::with_capacity(MAX_FRAMES);
        let mut unwind_error = None;
        let mut reached_stack_end = false;
        while ips.len() < MAX_FRAMES {
            let mut ip = 0;
            let get_reg_result = unsafe { (libunwind.get_reg)(&mut cursor, UNW_REG_IP, &mut ip) };
            if get_reg_result < 0 {
                unwind_error = Some(anyhow!(
                    "libunwind failed to read instruction pointer after {} frames \
                     (error {get_reg_result})",
                    ips.len()
                ));
                break;
            }
            if ip == 0 {
                reached_stack_end = true;
                break;
            }
            ips.push(ip as usize as *mut c_void);

            let step_result = unsafe { (libunwind.step)(&mut cursor) };
            if step_result == 0 {
                reached_stack_end = true;
                break;
            }
            if step_result == -UNW_EINVAL && !ips.is_empty() {
                reached_stack_end = true;
                break;
            }
            if step_result < 0 {
                unwind_error = Some(anyhow!(
                    "libunwind stopped after {} frames (error {step_result}); \
                     captured {} stack bytes",
                    ips.len(),
                    snapshot.stack.len()
                ));
                break;
            }
        }

        unsafe { (libunwind.destroy_addr_space)(address_space) };
        if let Some(error) = unwind_error {
            return Err(error);
        }
        if !reached_stack_end {
            return Err(anyhow!(
                "Native stack exceeds the maximum of {MAX_FRAMES} frames"
            ));
        }
        Ok(ips)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn snapshot_metadata_fits_in_pipe_buf() {
            assert!(size_of::<SnapshotPrefix>() < 4096);
            assert!(MAX_STACK_SNAPSHOT_SIZE > 4096);
        }

        #[inline(never)]
        fn stack_level_three(
            sender: std::sync::mpsc::Sender<i32>,
            done: std::sync::Arc<std::sync::atomic::AtomicBool>,
        ) {
            let tid = unsafe { libc::syscall(libc::SYS_gettid) as i32 };
            sender.send(tid).unwrap();
            while !done.load(Ordering::Acquire) {
                std::hint::spin_loop();
            }
        }

        #[inline(never)]
        fn stack_level_two(
            sender: std::sync::mpsc::Sender<i32>,
            done: std::sync::Arc<std::sync::atomic::AtomicBool>,
        ) {
            stack_level_three(sender, done);
            std::hint::black_box(());
        }

        #[inline(never)]
        fn stack_level_one(
            sender: std::sync::mpsc::Sender<i32>,
            done: std::sync::Arc<std::sync::atomic::AtomicBool>,
        ) {
            // Keep a frame larger than the old 2 KiB snapshot alive. Reaching
            // the caller requires the unwinder to read beyond that old limit.
            let padding = [0xa5_u8; 8192];
            std::hint::black_box(&padding);
            stack_level_two(sender, done);
            std::hint::black_box(padding[0]);
        }

        #[test]
        fn captures_complete_stack_beyond_old_two_kib_limit() {
            let (sender, receiver) = std::sync::mpsc::channel();
            let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let target_done = done.clone();
            let target = std::thread::spawn(move || stack_level_one(sender, target_done));
            let tid = receiver.recv().unwrap();
            let pid = unsafe { libc::getpid() };

            let result = capture_raw_frames(pid, tid, 2000);
            done.store(true, Ordering::Release);
            target.join().unwrap();

            let frames = result.unwrap();
            assert!(
                frames.len() >= 6,
                "expected a complete native stack beyond 8 KiB, got {} frames",
                frames.len()
            );
        }

        #[test]
        fn locates_mapping_containing_stack_pointer() {
            let pid = unsafe { libc::getpid() };
            let tid = unsafe { libc::syscall(libc::SYS_gettid) as i32 };
            let local = 0_u8;
            let address = ptr::from_ref(&local) as u64;
            let (start, end) = find_stack_mapping(pid, tid, address).unwrap();
            assert!(address >= start && address < end);
        }

        #[test]
        fn rejects_invalid_snapshot_record() {
            let invalid = Snapshot {
                prefix: SnapshotPrefix {
                    magic: 0,
                    version: 0,
                    register_count: 0,
                    stack_size: 0,
                    reserved: 0,
                    stack_base: 0,
                    registers: [0; SNAPSHOT_REGISTER_COUNT],
                },
                stack: &[],
                mapping_start: 0,
                mapping_end: 0,
            };
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
