pub(crate) mod python_bindings;

pub(crate) mod python_interpreters;

pub(crate) mod call;
pub(crate) mod ffi;

pub use python_bindings::version::Version;

use crate::features::spy::{call::RawCallLocation, python_interpreters::{BytesObject, CodeObject, StringObject}};

pub(crate) static mut PYVERSION: Version = Version {
    major: 0,
    minor: 0,
    patch: 0,
    release_flags: String::new(),
    build_metadata: None,
};

#[thread_local]
pub(crate) static mut PYSTACKS: Vec<RawCallLocation> = Vec::new();

#[thread_local]
pub(crate) static mut PYFRAMEEVAL: ffi::_PyFrameEvalFunction = ffi::_PyEval_EvalFrameDefault;

/// 获取当前线程执行的Python frame指针
/// 这个函数适用于在信号处理函数中调用
#[inline(always)]
pub unsafe fn get_current_frame(ver: &Version) -> Option<usize> {
    // 获取当前线程状态
    let threadstate: usize = get_current_threadstate()?;

    match (ver.major, ver.minor) {
        (3, 4) | (3, 5) | (3, 6) | (3, 7) | (3, 8) | (3, 9) | (3, 10) => {
            // Python 3.4 to 3.10
            let ts = threadstate as *const super::spy::python_bindings::v3_10_0::PyThreadState;
            let frame = unsafe { (*ts).frame };
            if !frame.is_null() {
                Some(frame as usize)
            } else {
                None
            }
        }
        (3, 11) => {
            // Python 3.11
            let ts = threadstate as *const super::spy::python_bindings::v3_11_0::PyThreadState;
            let cframe = unsafe { (*ts).cframe };
            if !cframe.is_null() {
                let current_frame = (*cframe).current_frame;
                if !current_frame.is_null() {
                    Some(current_frame as usize)
                } else {
                    None
                }
            } else {
                None
            }
        }
        (3, 12) => {
            // Python 3.12
            let ts = threadstate as *const super::spy::python_bindings::v3_12_0::PyThreadState;
            let cframe = unsafe { (*ts).cframe };
            if !cframe.is_null() {
                let current_frame = (*cframe).current_frame;
                if !current_frame.is_null() {
                    Some(current_frame as usize)
                } else {
                    None
                }
            } else {
                None
            }
        }
        (3, 13) => {
            // Python 3.13
            let ts = threadstate as *const super::spy::python_bindings::v3_13_0::PyThreadState;
            let current_frame = unsafe { (*ts).current_frame };
            if !current_frame.is_null() {
                Some(current_frame as usize)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[inline(always)]
pub fn get_next_frame(ver: &Version, frame_addr: usize) -> Option<usize> {
    match (ver.major, ver.minor) {
        (3, 4) | (3, 5) | (3, 6) | (3, 7) | (3, 8) | (3, 9) | (3, 10) => {
            let frame = frame_addr as *const super::spy::python_bindings::v3_10_0::_frame;
            let next_frame = unsafe { (*frame).f_back };
            if !next_frame.is_null() {
                Some(next_frame as usize)
            } else {
                None
            }
        }
        (3, 11) => {
            let iframe =
                frame_addr as *const super::spy::python_bindings::v3_11_0::_PyInterpreterFrame;
            let next_frame = unsafe { (*iframe).previous };
            if !next_frame.is_null() {
                Some(next_frame as usize)
            } else {
                None
            }
        }
        (3, 12) => {
            let iframe =
                frame_addr as *const super::spy::python_bindings::v3_12_0::_PyInterpreterFrame;
            let next_frame = unsafe { (*iframe).previous };
            if !next_frame.is_null() {
                Some(next_frame as usize)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// 获取当前线程的PyThreadState指针
/// 这个函数使用Python C API来获取当前线程状态
#[inline(always)]
pub unsafe fn get_current_threadstate() -> Option<usize> {
    extern "C" {
        fn PyThreadState_Get() -> *mut std::ffi::c_void;
    }

    let threadstate = PyThreadState_Get();
    if !threadstate.is_null() {
        Some(threadstate as usize)
    } else {
        None
    }
}

/// 获取当前执行的Python frame信息
/// 这个函数结合了获取当前frame和解析frame的功能
#[inline(always)]
pub unsafe fn get_current_frame_info(ver: &Version) -> Option<(usize, i32)> {
    if let Some(frame_addr) = get_current_frame(ver) {
        let (code, lineno) = parse_frame(ver, frame_addr);
        if code != 0 {
            Some((code, lineno))
        } else {
            None
        }
    } else {
        None
    }
}

#[inline(always)]
pub unsafe fn parse_frame(ver: &Version, addr: usize) -> (usize, i32) {
    match (ver.major, ver.minor) {
        (3, 4) | (3, 5) | (3, 6) | (3, 7) | (3, 8) | (3, 9) | (3, 10) => {
            // Python 3.4 to 3.9
            let frame = addr as *const super::spy::python_bindings::v3_10_0::_frame;
            let code = unsafe { (*frame).f_code };
            let lasti = unsafe { (*frame).f_lasti };
            (code as usize, lasti)
        }
        (3, 11) => {
            let iframe = addr as *const super::spy::python_bindings::v3_11_0::_PyInterpreterFrame;
            unsafe {
                let code = (*iframe).f_code;
                let lasti = ((*iframe).prev_instr as *const u8).offset_from(code as *const u8);
                (code as usize, lasti as i32)
            }
        }
        (3, 12) => {
            // Python 3.10 and later
            let iframe = addr as *const super::spy::python_bindings::v3_12_0::_PyInterpreterFrame;
            unsafe {
                let code = (*iframe).f_code;
                let lasti = ((*iframe).prev_instr as *const u8).offset_from(code as *const u8);
                (code as usize, lasti as i32)
            }
        }
        _ => (0, 0),
    }
}

#[inline(always)]
pub unsafe fn parse_location(ver: &Version, code: usize, lasti: i32) -> (String, String, i32) {
    match (ver.major, ver.minor) {
        (3, 4) | (3, 5) | (3, 6) | (3, 7) | (3, 8) | (3, 9) | (3, 10) => {
            let code = code as *const super::spy::python_bindings::v3_11_0::PyCodeObject;
            parse_location_raw(code, lasti)
        }
        (3, 11) => {
            let code = code as *const super::spy::python_bindings::v3_11_0::PyCodeObject;
            parse_location_raw(code, lasti)
        }
        (3, 12) => {
            let code = code as *const super::spy::python_bindings::v3_12_0::PyCodeObject;
            parse_location_raw(code, lasti)
        }
        (3, 13) => {
            let code = code as *const super::spy::python_bindings::v3_13_0::PyCodeObject;
            parse_location_raw(code, lasti)
        }
        _ => Default::default(),
    }
}

unsafe fn parse_location_raw<T: CodeObject>(code: *const T, lasti: i32) -> (String, String, i32) {
    let filename = (*code).filename();
    let funcname = (*code).name();
    let line_table_ptr = (*code).line_table();
    let line_table_size = (*line_table_ptr).size();

    let mut line_table_bytes: Vec<u8> = Vec::with_capacity(line_table_size);
    std::ptr::copy_nonoverlapping(
        line_table_ptr as *const _,
        line_table_bytes.as_mut_ptr(),
        line_table_size,
    );
    line_table_bytes.set_len(line_table_size);

    let filename = copy_string(
        (*filename).address(filename as usize) as *const u8,
        (*filename).size() * (*filename).kind() as usize,
        (*filename).kind(),
        (*filename).ascii(),
    );

    let funcname = copy_string(
        (*funcname).address(funcname as usize) as *const u8,
        (*funcname).size() * (*funcname).kind() as usize,
        (*funcname).kind(),
        (*funcname).ascii(),
    );
    let lineno = (*code).get_line_number(lasti, line_table_bytes.as_slice());
    (filename, funcname, lineno)
}

fn copy_string(addr: *const u8, len: usize, kind: u32, ascii: bool) -> String {
    let len = if len > 1024 { 1024 } else { len };
    match (kind, ascii) {
        (4, _) => {
            let chars = unsafe { std::slice::from_raw_parts(addr as *const char, len / 4) };
            chars.iter().collect()
        }
        (2, _) => {
            let chars = unsafe { std::slice::from_raw_parts(addr as *const u16, len / 2) };
            String::from_utf16(chars).unwrap_or_default()
        }
        (1, true) => {
            let slice = unsafe { std::slice::from_raw_parts(addr, len) };
            String::from_utf8_lossy(slice).to_string()
        }
        (1, false) => {
            let slice = unsafe { std::slice::from_raw_parts(addr, len) };
            String::from_utf8_lossy(slice).to_string()
        }
        _ => String::new(),
    }
}

/// 从threadstate获取当前frame
#[inline(always)]
pub unsafe fn get_frame_from_threadstate(ver: &Version, threadstate_addr: usize) -> Option<usize> {
    match (ver.major, ver.minor) {
        (3, 4) | (3, 5) | (3, 6) | (3, 7) | (3, 8) | (3, 9) | (3, 10) => {
            let ts = threadstate_addr as *const super::spy::python_bindings::v3_10_0::PyThreadState;
            let frame = (*ts).frame;
            if !frame.is_null() {
                Some(frame as usize)
            } else {
                None
            }
        }
        (3, 11) => {
            let ts = threadstate_addr as *const super::spy::python_bindings::v3_11_0::PyThreadState;
            let cframe = (*ts).cframe;
            if !cframe.is_null() {
                let cframe_obj = &*cframe;
                let current_frame = cframe_obj.current_frame;
                if !current_frame.is_null() {
                    Some(current_frame as usize)
                } else {
                    None
                }
            } else {
                None
            }
        }
        (3, 12) => {
            let ts = threadstate_addr as *const super::spy::python_bindings::v3_12_0::PyThreadState;
            let cframe = (*ts).cframe;
            if !cframe.is_null() {
                let cframe_obj = &*cframe;
                let current_frame = cframe_obj.current_frame;
                if !current_frame.is_null() {
                    Some(current_frame as usize)
                } else {
                    None
                }
            } else {
                None
            }
        }
        (3, 13) => {
            let ts = threadstate_addr as *const super::spy::python_bindings::v3_13_0::PyThreadState;
            let current_frame = (*ts).current_frame;
            if !current_frame.is_null() {
                Some(current_frame as usize)
            } else {
                None
            }
        }
        _ => None,
    }
}
