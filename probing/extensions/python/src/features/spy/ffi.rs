use core::ffi::c_int;

use pyo3::ffi::PyFrameObject;
use pyo3::ffi::PyInterpreterState;
use pyo3::ffi::PyObject;
use pyo3::ffi::PyThreadState;

pub type _PyFrameEvalFunction =
    unsafe extern "C" fn(*mut PyThreadState, *mut PyFrameObject, c_int) -> *mut pyo3::ffi::PyObject;

extern "C" {
    /// Get the frame evaluation function.
    pub fn _PyInterpreterState_GetEvalFrameFunc(
        interp: *mut PyInterpreterState,
    ) -> _PyFrameEvalFunction;

    ///Set the frame evaluation function.
    pub fn _PyInterpreterState_SetEvalFrameFunc(
        interp: *mut pyo3::ffi::PyInterpreterState,
        eval_frame: _PyFrameEvalFunction,
    );

    pub fn PyInterpreterState_Get() -> *mut PyInterpreterState;

    pub fn _PyEval_EvalFrameDefault(
        ts: *mut PyThreadState,
        frame: *mut PyFrameObject,
        extra: c_int,
    ) -> *mut PyObject;
}
