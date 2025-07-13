use core::ffi::c_int;

use pyo3::prelude::*;

use super::spy::{parse_frame, python_bindings};

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

static mut PYVERSION: python_bindings::version::Version = python_bindings::version::Version {
    major: 0,
    minor: 0,
    patch: 0,
    release_flags: String::new(),
    build_metadata: None,
};

#[thread_local]
static mut PYSTACKS: Vec<(u64, i32)> = Vec::new();

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
        let ret = EVAL_FRAME(ts, frame, extra);
        PYSTACKS.pop();
        ret
    }
}

#[thread_local]
static mut EVAL_FRAME: _PyFrameEvalFunction = rust_eval_frame;

#[allow(static_mut_refs)]
#[pyfunction]
pub fn enable_tracer() -> PyResult<()> {
    Python::with_gil(|py| {
        let ver = py.version_info();
        unsafe {
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
        }
    });
    unsafe {
        if PYVERSION.major != 0 {
            let interp = PyInterpreterState_Get();
            let old_eval_frame = _PyInterpreterState_GetEvalFrameFunc(interp);
            if old_eval_frame as usize != rust_eval_frame as usize {
                EVAL_FRAME = old_eval_frame;
            }
            _PyInterpreterState_SetEvalFrameFunc(interp, rust_eval_frame);
        }
    }
    Ok(())
}

#[allow(static_mut_refs)]
#[pyfunction]
pub fn disable_tracer() -> PyResult<()> {
    unsafe {
        let interp = PyInterpreterState_Get();
        let old_eval_frame = _PyInterpreterState_GetEvalFrameFunc(interp);
        if old_eval_frame as usize == rust_eval_frame as usize {
            _PyInterpreterState_SetEvalFrameFunc(interp, EVAL_FRAME);
        }
        PYSTACKS.clear();
        PYSTACKS.shrink_to_fit();
    }
    Ok(())
}
