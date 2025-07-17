use core::ffi::c_int;

use pyo3::prelude::*;

use probing_proto::prelude::CallFrame;

use crate::features::spy::{get_current_frame, get_next_frame, parse_location};

use super::spy::{parse_frame, python_bindings};
use python_bindings::version::Version;

mod ffi {
    use core::ffi::c_int;

    pub type _PyFrameEvalFunction = extern "C" fn(
        *mut pyo3::ffi::PyThreadState,
        *mut pyo3::ffi::PyFrameObject,
        c_int,
    ) -> *mut pyo3::ffi::PyObject;

    extern "C" {
        /// Get the frame evaluation function.
        pub fn _PyInterpreterState_GetEvalFrameFunc(
            interp: *mut pyo3::ffi::PyInterpreterState,
        ) -> _PyFrameEvalFunction;

        ///Set the frame evaluation function.
        pub fn _PyInterpreterState_SetEvalFrameFunc(
            interp: *mut pyo3::ffi::PyInterpreterState,
            eval_frame: _PyFrameEvalFunction,
        );

        pub fn PyInterpreterState_Get() -> *mut pyo3::ffi::PyInterpreterState;

        pub fn _PyEval_EvalFrameDefault(
            ts: *mut pyo3::ffi::PyThreadState,
            frame: *mut pyo3::ffi::PyFrameObject,
            extra: c_int,
        ) -> *mut pyo3::ffi::PyObject;
    }
}

static mut PYVERSION: Version = Version {
    major: 0,
    minor: 0,
    patch: 0,
    release_flags: String::new(),
    build_metadata: None,
};

#[thread_local]
static mut PYSTACKS: Vec<(u64, i32)> = Vec::new();

#[thread_local]
static mut PYFRAMEEVAL: ffi::_PyFrameEvalFunction = rust_eval_frame;

#[allow(static_mut_refs)]
pub fn initialize_globals() {
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
            }
            if PYSTACKS.capacity() == 0 {
                PYSTACKS.reserve(1024);
            }
        }
    });
}

#[allow(static_mut_refs)]
#[inline(always)]
extern "C" fn rust_eval_frame(
    ts: *mut pyo3::ffi::PyThreadState,
    frame: *mut pyo3::ffi::PyFrameObject,
    extra: c_int,
) -> *mut pyo3::ffi::PyObject {
    unsafe {
        let (code, lineno) = parse_frame(&PYVERSION, frame as usize);
        PYSTACKS.push((code as u64, lineno));
        let ret = PYFRAMEEVAL(ts, frame, extra);
        PYSTACKS.pop();
        ret
    }
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
    unsafe {
        let py_list = PyList::empty(py);

        for frame in get_python_frames_raw(&PYVERSION) {
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
            .map(|(code, lasti)| {
                let (filename, funcname, lineno) =
                    parse_location(&PYVERSION, *code as usize, *lasti);
                CallFrame::PyFrame {
                    file: filename,
                    func: funcname,
                    lineno: lineno as i64,
                    locals: Default::default(),
                }
            })
            .collect::<Vec<_>>()
    }
}

pub fn get_python_frames_raw(ver: &Version) -> Vec<CallFrame> {
    let mut frames = vec![];
    let mut current_frame_addr = unsafe { get_current_frame(ver) };
    while let Some(addr) = current_frame_addr {
        let (code, lasti) = unsafe { parse_frame(ver, addr) };
        if code != 0 {
            let (filename, funcname, lineno) = unsafe { parse_location(ver, code, lasti) };
            if filename != "<shim>" || funcname != "<interpreter trampoline>" {
                frames.push(CallFrame::PyFrame {
                    file: filename,
                    func: funcname,
                    lineno: lineno as i64,
                    locals: Default::default(),
                });
            }
            current_frame_addr = get_next_frame(ver, addr);
        }
    }

    frames
}
