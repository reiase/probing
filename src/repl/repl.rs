use crate::prof::PPROF;
use lazy_st::lazy;
use std::sync::{Arc, Mutex};

use super::console::SharedNativeConsole;
use super::{
    console::{NativePythonConsole, RustPythonConsole},
    npy_repl::NPYVM,
};

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
        println!("===========================");
        let has_native = NPYVM.lock().map(|vm| vm.is_some()).unwrap();
        if has_native {
            Self {
                console: Arc::new(Mutex::new(NativePythonConsole::default())),
                buf: Default::default(),
                live: Default::default(),
            }
        } else {
            Self {
                console: Arc::new(Mutex::new(RustPythonConsole::default())),
                buf: Default::default(),
                live: Default::default(),
            }
        }
        // Self {
        //     console: Arc::new(Mutex::new(RustPythonConsole::default())),
        //     buf: Default::default(),
        //     live: Default::default(),
        // }
    }
}

impl PythonRepl {
    #[inline(never)]
    fn native() -> Self {
        Self {
            console: SharedNativeConsole.clone(),
            buf: Default::default(),
            live: Default::default(),
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
    pub fn process(&mut self, cmd: &str) -> Option<String> {
        if cmd.trim() == "exit" {
            self.live = false;
            return None;
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
