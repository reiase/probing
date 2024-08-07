use std::{ffi::c_void, ptr::null, sync::Mutex};

use anyhow::Result;
use nix::libc::{dlopen, dlsym, RTLD_LAZY};
use once_cell::sync::Lazy;

pub static RAW_ALLOC_PTR: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(0));
pub static RAW_DELETE_PTR: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(0));

pub fn init_alloc_hook() -> Result<()> {
    let error_msg = "unable to open resolve symbol for raw_alloc/raw/free";

    unsafe {
        let handle = dlopen(null(), RTLD_LAZY);
        if handle as u64 == 0 {
            return Err(anyhow::anyhow!(error_msg));
        }
        let symbol = dlsym(
            handle,
            "_ZN3c104cuda20CUDACachingAllocator6Native22NativeCachingAllocator9raw_allocEm\0"
                .as_bytes()
                .as_ptr() as *const i8,
        );
        if symbol as u64 == 0 {
            return Err(anyhow::anyhow!(error_msg));
        }
        RAW_ALLOC_PTR
            .lock()
            .map(|mut x| *x = symbol as u64)
            .unwrap();
        let symbol = dlsym(
            handle,
            "_ZNK3c104cuda20CUDACachingAllocator6Native22NativeCachingAllocator11raw_deleterEv\0"
                .as_bytes()
                .as_ptr() as *const i8,
        );
        if symbol as u64 == 0 {
            return Err(anyhow::anyhow!(error_msg));
        }
        RAW_DELETE_PTR
            .lock()
            .map(|mut x| *x = symbol as u64)
            .unwrap();
    }
    Ok(())
}

#[no_mangle]
pub extern "C" fn raw_alloc_hook(size: u64) -> *mut c_void {
    let mut func = RAW_ALLOC_PTR.lock().map(|x| *x).unwrap();
    println!("hooked raw alloc");
    if func == 0 {
        init_alloc_hook().unwrap();
        func = RAW_ALLOC_PTR.lock().map(|x| *x).unwrap();
    }
    unsafe {
        let func: fn(u64) -> *mut c_void = std::mem::transmute(func);
        func(size)
    }
}

#[no_mangle]
pub extern "C" fn raw_delete_hook(ptr: *mut c_void) {
    let mut func = RAW_DELETE_PTR.lock().map(|x| *x).unwrap();
    println!("hooked raw delete");
    if func == 0 {
        init_alloc_hook().unwrap();
        func = RAW_DELETE_PTR.lock().map(|x| *x).unwrap();
    }
    unsafe {
        let func: fn(*mut c_void) = std::mem::transmute(func);
        func(ptr)
    }
}
