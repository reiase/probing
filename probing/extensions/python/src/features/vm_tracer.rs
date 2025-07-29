use core::ffi::c_int;

use pyo3::prelude::*;

use probing_proto::prelude::CallFrame;

use crate::features::spy::call::RawCallLocation;
use crate::features::spy::{get_current_frame, get_prev_frame};

use super::spy::python_bindings;

use crate::features::spy::ffi;
use crate::features::spy::PYFRAMEEVAL;
use crate::features::spy::PYSTACKS;
use crate::features::spy::PYVERSION;

#[allow(static_mut_refs)]
pub fn initialize_globals() -> bool {
    Python::with_gil(|py| {
        let ver = py.version_info();
        unsafe {
            if PYVERSION.major == 0 {
                PYVERSION = python_bindings::version::Version {
                    major: ver.major as u64,
                    minor: ver.minor as u64,
                    patch: ver.patch as u64,
                    release_flags: ver.suffix.unwrap_or_default().to_string(),
                    build_metadata: Default::default(),
                };
                if PYSTACKS.capacity() == 0 {
                    PYSTACKS.reserve(1024);
                }
                true
            } else {
                false
            }
        }
    })
}

#[allow(static_mut_refs)]
#[inline(always)]
unsafe extern "C" fn rust_eval_frame(
    ts: *mut pyo3::ffi::PyThreadState,
    frame: *mut pyo3::ffi::PyFrameObject,
    extra: c_int,
) -> *mut pyo3::ffi::PyObject {
    PYSTACKS.push(RawCallLocation::from(frame as usize, Some(ts as usize)));
    let ret = PYFRAMEEVAL(ts, frame, extra);
    PYSTACKS.pop();
    ret
}

#[allow(static_mut_refs)]
#[pyfunction]
pub fn enable_tracer() -> PyResult<()> {
    unsafe {
        if PYVERSION.major == 3 && PYVERSION.minor >= 10 {
            let interp = ffi::PyInterpreterState_Get();
            let old_eval_frame = ffi::_PyInterpreterState_GetEvalFrameFunc(interp);
            if old_eval_frame as usize != rust_eval_frame as usize {
                PYFRAMEEVAL = old_eval_frame;
            }
            ffi::_PyInterpreterState_SetEvalFrameFunc(interp, rust_eval_frame);
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Python version {}.{} does not support tracer",
                PYVERSION.major, PYVERSION.minor
            )));
        }
    }
    Ok(())
}

#[allow(static_mut_refs)]
#[pyfunction]
pub fn disable_tracer() -> PyResult<()> {
    unsafe {
        let interp = ffi::PyInterpreterState_Get();
        let old_eval_frame = ffi::_PyInterpreterState_GetEvalFrameFunc(interp);
        if old_eval_frame as usize == rust_eval_frame as usize {
            ffi::_PyInterpreterState_SetEvalFrameFunc(interp, PYFRAMEEVAL);
        }
        PYSTACKS.clear();
        PYSTACKS.shrink_to_fit();
    }
    Ok(())
}

#[pyfunction]
pub fn _get_python_stacks(py: Python) -> PyResult<PyObject> {
    use pyo3::types::{PyDict, PyList};

    let py_list = PyList::empty(py);
    for frame in get_python_stacks_raw() {
        if let CallFrame::PyFrame {
            file, func, lineno, ..
        } = frame
        {
            let dict = PyDict::new(py);
            dict.set_item("file", file)?;
            dict.set_item("func", func)?;
            dict.set_item("lineno", lineno)?;
            py_list.append(dict)?;
        }
    }
    Ok(py_list.into())
}

#[allow(static_mut_refs)]
#[pyfunction]
pub fn _get_python_frames(py: Python) -> PyResult<PyObject> {
    use pyo3::types::{PyDict, PyList};

    let py_list = PyList::empty(py);

    for frame in get_python_frames_raw(None) {
        if let CallFrame::PyFrame {
            file, func, lineno, ..
        } = frame
        {
            let dict = PyDict::new(py);
            dict.set_item("file", file)?;
            dict.set_item("func", func)?;
            dict.set_item("lineno", lineno)?;
            py_list.append(dict)?;
        }
    }
    Ok(py_list.into())
}

#[allow(static_mut_refs)]
pub fn get_python_stacks_raw() -> Vec<CallFrame> {
    unsafe {
        if PYSTACKS.capacity() == 0 {
            return vec![];
        }
        PYSTACKS
            .iter()
            .rev()
            .map(|location| {
                let location = location.resolve().unwrap_or_default();
                CallFrame::PyFrame {
                    file: location.callee.file,
                    func: location.callee.name,
                    lineno: location.callee.line as i64,
                    locals: Default::default(),
                }
            })
            .collect::<Vec<_>>()
    }
}

#[allow(static_mut_refs)]
pub fn get_python_frames_raw(current_frame: Option<usize>) -> Vec<CallFrame> {
    let mut frames = vec![];
    let mut current_frame_addr = match current_frame {
        Some(addr) => Some(addr),
        None => unsafe { get_current_frame(&PYVERSION) },
    };

    if let Some(addr) = current_frame_addr {
        let location = RawCallLocation::from(addr, None).resolve();
        log::debug!("Current frame address: {addr:#x}, location: {location:?}");
        if let Ok(location) = location {
            let filename = location.callee.file;
            let funcname = location.callee.name;
            if filename != "<shim>" || funcname != "<interpreter trampoline>" {
                frames.push(CallFrame::PyFrame {
                    file: filename,
                    func: funcname,
                    lineno: location.callee.line as i64,
                    locals: Default::default(),
                });
            }
        }
    }

    while let Some(addr) = current_frame_addr {
        let location = RawCallLocation::from(addr, None).resolve();
        log::debug!("Current frame address: {addr:#x}, location: {location:?}");
        if let Ok(location) = location {
            if let Some(caller) = location.caller {
                if caller.file != "<shim>" && caller.name != "<interpreter trampoline>" {
                    frames.push(CallFrame::PyFrame {
                        file: caller.file,
                        func: caller.name,
                        lineno: location.lineno as i64,
                        locals: Default::default(),
                    });
                }
            }
            current_frame_addr = unsafe { get_prev_frame(&PYVERSION, addr) };
        }
    }
    frames
}
