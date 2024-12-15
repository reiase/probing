use anyhow::Result;

use crate::repl::PythonRepl;

pub fn handle(code: String) -> Result<String> {
    let mut repl = PythonRepl::default();
    Ok(repl.process(code.as_str()).unwrap_or_default())
}
