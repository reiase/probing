use crate::repl::console::NativePythonConsole;
use std::sync::{Arc, Mutex};

pub trait Repl {
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
        self.console.lock().unwrap().try_execute(cmd.to_string())
    }
}

impl Repl for PythonRepl {
    fn feed(&mut self, s: String) -> Option<String> {
        self.buf += &s;
        if !self.buf.contains('\n') {
            return None;
        }
        match self.buf.rsplit_once('\n') {
            Some((cmd, rest)) => {
                let cmd = cmd.to_string();
                self.buf = rest.to_string();
                self.process(cmd.as_str())
            }
            None => None,
        }
    }

    fn is_alive(&self) -> bool {
        self.live
    }
}
