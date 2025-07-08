use core::ffi::c_int;

use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::extensions;
use crate::features::spy::parse_frame;
use crate::pkg::TCPStore;
use probing_core::ENGINE;

#[pyfunction]
fn query_json(_py: Python, sql: String) -> PyResult<String> {
    let result = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { ENGINE.read().await.async_query(sql.as_str()).await })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    serde_json::to_string(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

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

static mut PYVERSION: super::spy::python_bindings::version::Version =
    super::spy::python_bindings::version::Version {
        major: 0,
        minor: 0,
        patch: 0,
        release_flags: String::new(),
        build_metadata: None,
    };

#[thread_local]
static mut PYSTACKS: Vec<(u64, i32)> = Vec::new();

#[inline(always)]
extern "C" fn rust_eval_frame(
    ts: *mut pyo3::ffi::PyThreadState,
    frame: *mut pyo3::ffi::PyFrameObject,
    extra: c_int,
) -> *mut pyo3::ffi::PyObject {
    unsafe {
        let (code, lineno) = if PYVERSION.major != 0 {
            parse_frame(&PYVERSION, frame as usize)
        } else {
            (0usize, 0i32)
        };
        PYSTACKS.push((code as u64, lineno));
        let ret = EVAL_FRAME(ts, frame, extra);
        PYSTACKS.pop();
        ret
    }
}

#[thread_local]
static mut EVAL_FRAME: _PyFrameEvalFunction = rust_eval_frame;

#[pyfunction]
fn enable_tracer() -> PyResult<()> {
    Python::with_gil(|py| {
        let ver = py.version_info();
        unsafe {
            PYVERSION = super::spy::python_bindings::version::Version {
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
        let interp = PyInterpreterState_Get();
        let old_eval_frame = _PyInterpreterState_GetEvalFrameFunc(interp);
        if old_eval_frame as usize != rust_eval_frame as usize {
            EVAL_FRAME = old_eval_frame;
        }
        _PyInterpreterState_SetEvalFrameFunc(interp, rust_eval_frame);
    }
    Ok(())
}

#[pyfunction]
fn disable_tracer() -> PyResult<()> {
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

pub fn create_probing_module() -> PyResult<()> {
    Python::with_gil(|py| -> PyResult<()> {
        let sys = PyModule::import(py, "sys")?;
        let modules = sys.getattr("modules")?;

        if !modules.contains("probing")? {
            let m = PyModule::new(py, "probing")?;
            modules.set_item("probing", m)?;
        }

        let m = PyModule::import(py, "probing")?;
        if m.hasattr(pyo3::intern!(py, "_C"))? {
            return Ok(());
        }
        m.setattr(pyo3::intern!(py, "_C"), 42)?;
        m.add_class::<extensions::python::ExternalTable>()?;
        m.add_class::<TCPStore>()?;
        m.add_function(wrap_pyfunction!(query_json, py)?)?;
        m.add_function(wrap_pyfunction!(enable_tracer, py)?)?;
        m.add_function(wrap_pyfunction!(disable_tracer, py)?)?;
        Ok(())
    })
}
