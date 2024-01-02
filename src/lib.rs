#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

mod repl;
mod server;

use lazy_static::lazy_static;
use repl::REPL;
use std::{env, io::Error, thread};

use pprof::ProfilerGuard;
use pprof::ProfilerGuardBuilder;
use rustpython::vm::{AsObject, Interpreter, PyObjectRef};
use std::sync::Mutex;

use server::start_async_server;
use server::start_debug_server;

struct PyVM {
    interp: Interpreter,
    scope: PyObjectRef,
}

lazy_static! {
    static ref PYVM: Mutex<PyVM> = Mutex::new({
        let interp = rustpython::InterpreterConfig::new()
            .init_stdlib()
            .interpreter();
        let scope = interp.enter(|vm| {
            let scope = vm.new_scope_with_builtins();
            vm.run_block_expr(scope, repl::CODE).unwrap()
        });
        PyVM { interp, scope }
    });
    static ref PPROF: Mutex<ProfilerGuard<'static>> = Mutex::new({
        println!("installing pprof");
        ProfilerGuardBuilder::default()
            .frequency(10000)
            // .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .unwrap()
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

struct DebugRepl {
    console: Option<PyObjectRef>,
    buf: String,
    live: bool,
}

impl Default for DebugRepl {
    fn default() -> Self {
        Self {
            console: Some(create_console()),
            buf: "".to_string(),
            live: true,
        }
    }
}

impl DebugRepl {
    pub fn new(console: Option<PyObjectRef>) -> DebugRepl {
        DebugRepl {
            console,
            buf: "".to_string(),
            live: true,
        }
    }
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
            Some("/flamegraph") => PPROF
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
    fn process(&mut self, cmd: &str) -> Option<String> {
        if cmd.trim() == "exit" {
            self.live = false
        }
        if cmd.trim() == "pprof" {
            return PPROF
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

impl REPL for DebugRepl {
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

pub fn debug_callback(addr: Option<String>) {
    let console = create_console();
    let mut repl = DebugRepl::new(Some(console));
    start_debug_server(addr, &mut repl);
}

pub fn enable_debug_server(
    addr: Option<String>,
    background: bool,
    pprof: bool,
) -> Result<(), Error> {
    unsafe {
        let tmp = addr.clone();
        signal_hook::low_level::register(signal_hook::consts::SIGUSR1, move || {
            debug_callback(tmp.clone())
        })?;
        let tmp = addr.clone();
        signal_hook::low_level::register(signal_hook::consts::SIGABRT, move || {
            debug_callback(tmp.clone())
        })?;
    }
    if background {
        thread::spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(start_async_server::<DebugRepl>(addr))
                .unwrap();
        });
    }
    if pprof {
        let _ = PPROF.lock().map(|pp| {});
    }
    Ok(())
}

#[ctor]
fn init() {
    println!("loading profguard\n");
    let _ = enable_debug_server(
        env::var("PROBE_ADDR").ok(),
        env::var("PROBE_BG").map(|_| true).unwrap_or(false),
        env::var("PROBE_PPROF").map(|_| true).unwrap_or(false),
    );
    let _ = PYVM.lock().map(|pyvm| {
        pyvm.interp.enter(|vm| {
            let scope = vm.new_scope_with_builtins();
            let _ = vm.run_block_expr(scope, "print('profguard has been loaded')");
        })
    });
}
