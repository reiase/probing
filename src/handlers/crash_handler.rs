use crate::repl::RustPythonRepl;
use crate::server::start_debug_server;

pub fn crash_handler(addr: Option<String>) {
    let mut repl = RustPythonRepl::default();
    start_debug_server(addr, &mut repl);
}
