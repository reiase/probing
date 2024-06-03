use backtrace;
use nu_ansi_term::Color;
use pyo3::{types::PyAnyMethods, Python, ToPyObject};

pub fn dump_stack() {
    eprintln!(
        "{}",
        Color::Red
            .bold()
            .paint("Python Runtime is found, dump python stack:"),
    );
    Python::with_gil(|py| {
        // let _ = py.run_bound("import traceback; traceback.print_stack()", None, None);
        let mut ret = Python::with_gil(|py| {
            let ret = py
                .import_bound("traceback")
                .unwrap()
                .call_method0("format_stack")
                .unwrap_or_else(|err| {
                    err.print(py);
                    err.to_string().to_object(py).into_bound(py)
                });
            let ret = "\n"
                .to_object(py)
                .call_method1(py, "join", (ret.as_unbound(),));
            match ret {
                Ok(obj) => obj.to_string(),
                Err(err) => {
                    err.print(py);
                    err.to_string()
                }
            }
        });

        const HEX_WIDTH: usize = 20;
        let mut cnt = 0;
        backtrace::trace(|frame| {
            let ip = frame.ip();
            ret.push_str(format!("frame #{:<2} - {:#02$x}", cnt, ip as usize, HEX_WIDTH).as_str());
            cnt += 1;

            let mut resolved = false;
            backtrace::resolve(frame.ip(), |symbol| {
                if !resolved {
                    resolved = true;
                } else {
                    ret.push_str(format!("{}", vec![" "; 7 + 2 + 3 + HEX_WIDTH].join("")).as_str());
                }

                if let Some(name) = symbol.name() {
                    ret.push_str(format!(" - {name}").as_str());
                } else {
                    ret.push_str(" - <unknown>");
                }
                if let Some(file) = symbol.filename() {
                    if let Some(l) = symbol.lineno() {
                        ret.push_str(
                            format!("\n{:13}{:4$}@ {}:{}", "", "", file.display(), l, HEX_WIDTH)
                                .as_str(),
                        );
                    }
                }
                ret.push('\n');
            });
            if !resolved {
                println!(" - <no info>");
            }
            true // keep going
        });

        eprintln!("{}", ret);
    });
}
