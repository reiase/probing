use crate::handlers::PPROF_HOLDER;
use crate::repl::console::NativePythonConsole;
use std::sync::{Arc, Mutex};

pub trait REPL {
    fn feed(&mut self, s: String) -> Option<String>;
    fn is_alive(&self) -> bool;
}

pub trait PythonConsole {
    fn try_execute(&mut self, cmd: String) -> Option<String>;
}

pub struct PythonRepl {
    console: Arc<Mutex<dyn PythonConsole + Send>>,
    buf: String,
    live: bool,
}

impl Default for PythonRepl {
    #[inline(never)]
    fn default() -> Self {
        Self {
            console: Arc::new(Mutex::new(NativePythonConsole::default())),
            buf: Default::default(),
            live: true,
        }
    }
}

impl PythonRepl {
    pub fn process(&mut self, cmd: &str) -> Option<String> {
        if cmd.trim() == "exit" {
            self.live = false;
            return None;
        }
        if cmd.trim() == "pprof" {
            return PPROF_HOLDER.report();
        }
        let ret = self.console.lock().unwrap().try_execute(cmd.to_string());
        ret
    }
}

impl REPL for PythonRepl {
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
