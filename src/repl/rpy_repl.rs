use lazy_static::lazy_static;

use rustpython::vm::{AsObject, Interpreter, PyObjectRef};
use std::sync::Mutex;

use crate::repl::REPL;

use super::console::CODE;
use crate::handlers::PPROF_HOLDER;

pub struct RPyVM {
    pub interp: Interpreter,
    pub scope: PyObjectRef,
}

lazy_static! {
    pub static ref PYVM: Mutex<RPyVM> = Mutex::new({
        let interp = rustpython::InterpreterConfig::new()
            .init_stdlib()
            .interpreter();
        let scope = interp.enter(|vm| {
            let scope = vm.new_scope_with_builtins();
            vm.run_block_expr(scope, CODE).unwrap()
        });
        RPyVM { interp, scope }
    });
}

fn create_console() -> PyObjectRef {
    PYVM.lock()
        .map(|pyvm| {
            pyvm.interp
                .enter(|vm| pyvm.scope.get_item("debug_console", vm).unwrap())
        })
        .unwrap()
}

pub struct RustPythonRepl {
    console: Option<PyObjectRef>,
    buf: String,
    live: bool,
}

impl Default for RustPythonRepl {
    fn default() -> Self {
        Self {
            console: Some(create_console()),
            buf: "".to_string(),
            live: true,
        }
    }
}

impl RustPythonRepl {
    fn make_response(&self, ctype: Option<&str>, content: Option<String>) -> Option<String> {
        content.map_or(Some("HTTP/1.1 404 OK".to_string()), |content| {
            if content.is_empty() {
                Some("HTTP/1.1 200 OK".to_string())
            } else {
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: {}\r\n\r\n{}",
                    content.len(),
                    ctype.unwrap(),
                    content
                );
                Some(response)
            }
        })
    }
    fn url_handler(&mut self, path: Option<&str>) -> Option<String> {
        match path {
            Some("/flamegraph") => PPROF_HOLDER
                .lock()
                .map(|pp| {
                    if let Ok(report) = pp.report().build() {
                        let mut graph: Vec<u8> = vec![];
                        report.flamegraph(&mut graph).unwrap();
                        String::from_utf8(graph).ok()
                    } else {
                        None
                    }
                })
                .map_or(Some("HTTP/1.1 500 OK".to_string()), |g| {
                    self.make_response(Some("image/svg+xml"), g)
                }),
            Some("/") => Some(format!(
                r#"
                <html>
                <body>
                <p><a href="/flamegraph">flamegraph</a></p>
                </body>
                </html>
                "#
            ))
            .map_or(Some("HTTP/1.1 500 OK".to_string()), |c| {
                self.make_response(Some("text/html"), Some(c))
            }),
            Some(&_) => Some("HTTP/1.1 404 OK".to_string()),
            None => Some("HTTP/1.1 404 OK".to_string()),
        }
    }
    pub fn process(&mut self, cmd: &str) -> Option<String> {
        if cmd.trim() == "exit" {
            self.live = false;
            return None;
        }
        if cmd.trim() == "pprof" {
            return PPROF_HOLDER
                .lock()
                .map(|pp| {
                    if let Ok(report) = pp.report().build() {
                        Some(format!("report: {:?}", &report))
                    } else {
                        None
                    }
                })
                .ok()?;
        }
        if cmd.starts_with("GET ") {
            let mut headers = [httparse::EMPTY_HEADER; 64];
            let mut req = httparse::Request::new(&mut headers);
            let ret = req.parse(cmd.as_bytes()).map_or(false, |s| s.is_complete());
            if ret {
                return self.url_handler(req.path);
            }
        }
        let ret = self.console.as_ref().map(|console| {
            PYVM.lock()
                .map(|pyvm| {
                    pyvm.interp.enter(|vm| {
                        let args = cmd.to_string();
                        let func = console.as_ref().get_attr("push", vm).unwrap();
                        let ret = func.call((args,), vm);
                        match ret {
                            Ok(obj) => {
                                if vm.is_none(&obj) {
                                    None
                                } else {
                                    Some(obj.str(vm).unwrap().to_string())
                                }
                            }
                            Err(err) => Some(err.as_object().str(vm).unwrap().to_string()),
                        }
                    })
                })
                .unwrap()
        });
        ret?
    }
}

impl REPL for RustPythonRepl {
    fn feed(&mut self, s: String) -> Option<String> {
        self.buf += &s;
        if self.buf.starts_with("GET ") {
            let req = self.buf.clone();
            if let Some(rsp) = self.process(req.as_str()) {
                self.buf = "".to_string();
                return Some(rsp);
            } else {
                return None;
            }
        }
        if !self.buf.contains('\n') {
            return None;
        }
        let cmd = match self.buf.split_once('\n') {
            Some((cmd, rest)) => {
                let cmd = cmd.to_string();
                self.buf = rest.to_string();
                Some(cmd)
            }
            None => None,
        };
        if let Some(cmd) = cmd {
            self.process(&cmd)
        } else {
            None
        }
    }

    fn is_alive(&self) -> bool {
        self.live
    }
}
