use crate::{repl::PythonRepl, server::start_debug_server};

pub fn pause_process(addr: Option<String>) {
    let mut repl = PythonRepl::default();
    start_debug_server(addr, &mut repl);
}
