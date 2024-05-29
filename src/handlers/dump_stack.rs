use nu_ansi_term::Color;
use pyo3::Python;

pub fn dump_stack() {
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
