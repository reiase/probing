use std::ffi::{c_int, c_void};

mod catch_alloctor;

type SigHandler = fn(c_int);

#[no_mangle]
pub extern "C" fn signal(sig: c_int, handler: *mut c_void) -> *mut c_void {
    let handler = handler as u64;
    unsafe {
        let _ = signal_hook_registry::register_unchecked(sig, move |_: &_| {
            let handler: SigHandler = std::mem::transmute(handler);
            handler(sig);
        });
    };
    0 as *mut c_void
}
