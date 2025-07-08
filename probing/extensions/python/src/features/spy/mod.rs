pub(crate) mod python_bindings;

pub use python_bindings::version::Version;

/// 获取当前线程执行的Python frame指针
/// 这个函数适用于在信号处理函数中调用
#[inline(always)]
pub unsafe fn get_current_frame(ver: &Version) -> Option<usize> {
    // 获取当前线程状态
    let threadstate: usize = match get_current_threadstate() {
        Some(ts) => ts,
        None => return None,
    };

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
                let cframe_obj = unsafe { &*cframe };
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
            // Python 3.12
            let ts = threadstate as *const super::spy::python_bindings::v3_12_0::PyThreadState;
            let cframe = unsafe { (*ts).cframe };
            if !cframe.is_null() {
                let cframe_obj = unsafe { &*cframe };
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
            let lineno = unsafe { (*frame).f_lineno };
            (code as usize, lineno)
        }
        (3, 11) | (3, 12) => {
            // Python 3.10 and later
            let iframe = addr as *const super::spy::python_bindings::v3_12_0::_PyInterpreterFrame;
            let code = unsafe { (*iframe).f_code };
            let lineno = unsafe { (*iframe).stacktop };
            (code as usize, lineno)
        }
        _ => (0, 0),
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
