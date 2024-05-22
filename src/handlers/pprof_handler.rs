use lazy_static::lazy_static;
use std::sync::Mutex;

use pprof::ProfilerGuard;
use pprof::ProfilerGuardBuilder;

lazy_static! {
    pub static ref PPROF_HOLDER: Mutex<ProfilerGuard<'static>> = Mutex::new({
        println!("installing pprof");
        ProfilerGuardBuilder::default()
            .frequency(10000)
            // .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .unwrap()
    });
}

pub fn pprof_handler() {
    let _ = PPROF_HOLDER.lock().map(|pp| {});
}
