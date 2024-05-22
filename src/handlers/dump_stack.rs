use nu_ansi_term::Color;
use pyo3::Python;

use crate::repl::npy_repl::NPYVM;
pub fn dump_stack() {
    let has_native = NPYVM.lock().map(|vm| vm.is_some()).unwrap();
    if has_native {
        eprintln!(
            "{}",
            Color::Red
                .bold()
                .paint("Python Runtime is found, dump python stack:"),
        );
        Python::with_gil(|py| {
            let _ = py.run_bound("import traceback; traceback.print_stack()", None, None);
        });
    }
    {}
}
