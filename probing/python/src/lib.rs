pub mod plugins;
pub mod repl;

use anyhow::Context;
use anyhow::Result;
use repl::PythonRepl;

use std::ffi::CStr;

use probing_core::{CallFrame, Probe};
use pyo3::ffi::c_str;
use pyo3::Python;

pub struct PythonProbe {}

const DUMP_STACK: &CStr = c_str!(
    r#"
stacks = []

import sys

curr = sys._getframe(2)
while curr is not None:
    stack = {
        "file": curr.f_code.co_filename,
        "func": curr.f_code.co_name,
        "lineno": curr.f_lineno,
        "locals": {
            k: _get_obj_repr(v, value=True) for k, v in curr.f_locals.items()
        },
    }
    stacks.append(stack)
    curr = curr.f_back
import json
json.dumps(stacks)
"#
);

impl Probe for PythonProbe {
    fn backtrace(depth: Option<i32>) -> Result<Vec<CallFrame>> {
        let frames = Python::with_gil(|py| match py.eval(DUMP_STACK, None, None) {
            Ok(frames) => Ok(frames.to_string()),
            Err(err) => Err(anyhow::anyhow!(
                "error extract call stacks {}",
                err.to_string()
            )),
        })?;
        serde_json::from_str::<Vec<CallFrame>>(frames.as_str())
            .with_context(|| "error deserializing dump stack result".to_string())
    }

    fn eval<T: Into<String>>(code: T) -> Result<String> {
        let code: String = code.into();
        let mut repl = PythonRepl::default();
        Ok(repl.process(code.as_str()).unwrap_or_default())
    }
}
