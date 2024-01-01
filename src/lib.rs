#[allow(unused_imports)]
#[macro_use]
extern crate ctor;

pub mod debug;

use lazy_static::lazy_static;
use std::{env, io::Error, thread};

use rustpython::vm::{AsObject, Interpreter, PyObjectRef};
use std::sync::Mutex;

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
            vm.run_block_expr(scope, debug::CODE).unwrap()
        });
        PyVM { interp, scope }
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
    fn process(&mut self, cmd: &str) -> Option<String> {
        if cmd.trim() == "exit".to_string() {
            self.live = false
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

impl debug::REPL for DebugRepl {
    fn feed(&mut self, s: String) -> Option<String> {
        self.buf += &s;
        if !self.buf.contains("\n") {
            return None;
        }
        let cmd = match self.buf.split_once("\n") {
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
    debug::start_debug_server(addr, &mut repl);
}

pub fn enable_debug_server(addr: Option<String>, background: bool) -> Result<(), Error> {
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
                .block_on(debug::start_async_server::<DebugRepl>(addr))
                .unwrap();
        });
    }
    Ok(())
}

#[ctor]
fn init() {
    println!("loading profguard\n");
    let _ = enable_debug_server(
        env::var("PGUARD_ADDR").ok(),
        env::var("PGUARD_BG").map(|_| true).unwrap_or(false),
    );
    let _ = PYVM.lock().map(|pyvm| {
        pyvm.interp.enter(|vm| {
            let scope = vm.new_scope_with_builtins();
            let _ = vm.run_block_expr(scope, "print('profguard has been loaded')");
        })
    });
}
