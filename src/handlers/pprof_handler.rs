use lazy_static::lazy_static;
use signal_hook::consts::*;
use std::sync::Mutex;

use pprof::ProfilerGuard;
use pprof::ProfilerGuardBuilder;

use crate::SIGMAP;

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
    // SIGMAP
    //     .lock()
    //     .map(|m| {
    //         if let Some(sigid) = m.get(&SIGPROF) {
    //             signal_hook::low_level::unregister(*sigid);
    //         }
    //     })
    //     .unwrap();

    let _ = PPROF_HOLDER.lock().map(|pp| {});
}
