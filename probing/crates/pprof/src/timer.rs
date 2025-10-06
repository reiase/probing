#[repr(C)]
#[derive(Debug)]
struct Timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[repr(C)]
#[derive(Debug)]
struct Itimerval {
    pub it_interval: Timeval,
    pub it_value: Timeval,
}

extern "C" {
    fn setitimer(which: i32, new_value: *const Itimerval, old_value: *mut Itimerval) -> i32;
}

fn setitimer_pprof(new_value: *const Itimerval) {
    let ret = unsafe { setitimer(2, new_value, std::ptr::null_mut()) };
    if ret != 0 {
        log::error!("Failed to setitimer: {}", std::io::Error::last_os_error());
    }
}

pub fn reset_pprof_timer(freq: i64) {
    let interval = if freq > 0 { 1_000_000 / freq } else { 0 };
    let itimerval = Itimerval {
        it_interval: Timeval {
            tv_sec: interval / 1_000_000,
            tv_usec: interval - (interval / 1_000_000) * 1_000_000,
        },
        it_value: Timeval {
            tv_sec: interval / 1_000_000,
            tv_usec: interval - (interval / 1_000_000) * 1_000_000,
        },
    };
    log::debug!(
        "Resetting pprof timer with interval: {} us, itimerval: {:?}",
        interval,
        itimerval
    );

    setitimer_pprof(&itimerval);
}
