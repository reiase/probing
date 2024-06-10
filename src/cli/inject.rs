use std::fs;

use anyhow::Result;
use argh::FromArgs;
use ptrace_inject::{Injector, Process};

/// Inject into target process
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "inject")]
pub struct InjectCommand {
    /// enable profiling
    #[argh(switch, short = 'P')]
    pprof: bool,

    /// enable handling target process crash
    #[argh(switch, short = 'c')]
    crash: bool,

    /// enable background server
    #[argh(switch, short = 'b')]
    background: bool,

    /// address for remote connection (e.g., 127.0.0.1:8080)
    #[argh(option, short = 'a')]
    address: Option<String>,
}

impl InjectCommand {
    pub fn run(&self, pid: i32, dll: &Option<std::path::PathBuf>) -> Result<()> {
        let mut argstr = "".to_string();
        if self.pprof {
            argstr.push_str(" -P");
        }
        if self.crash {
            argstr.push_str(" -c");
        }
        if self.background {
            argstr.push_str(" -b");
        }
        if let Some(addr) = &self.address {
            argstr.push_str(" -a ");
            argstr.push_str(addr.as_str());
        }
        let process = Process::get(pid as u32).unwrap();
        let soname = if let Some(path) = dll {
            Some(path.clone())
        } else if let Ok(_path) = fs::read_link("/proc/self/exe") {
            println!(
                "base path: {} : {}",
                _path.display(),
                _path.parent().unwrap().display()
            );
            _path.with_file_name("libprobe.so").into()
        } else {
            None
        };
        println!(
            "Injecting {} into process {} with arguments `{}`",
            soname.clone().unwrap().to_str().unwrap(),
            pid,
            argstr,
        );
        Injector::attach(process)
            .unwrap()
            .inject(&soname.unwrap(), Some(argstr.as_str()))
            .unwrap();
        Ok(())
    }
}
