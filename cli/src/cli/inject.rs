use std::fs;

use anyhow::Result;
use argh::FromArgs;
use probe_common::cli::ProbeCommand;
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
        let mut probe_commands = vec![];
        if self.pprof {
            probe_commands.push(ProbeCommand::Pprof);
        }
        if self.crash {
            probe_commands.push(ProbeCommand::CatchCrash);
        }
        if self.background {
            probe_commands.push(ProbeCommand::ListenRemote {
                address: self.address.clone(),
            });
        }
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

        let argstr = ron::to_string(&probe_commands).unwrap();
        println!(
            "Injecting {} into process {} with arguments `{}`",
            soname.clone().unwrap().to_str().unwrap(),
            pid,
            argstr,
        );

        let process = Process::get(pid as u32).unwrap();
        Injector::attach(process)
            .unwrap()
            .inject(&soname.unwrap(), Some(argstr.as_str()))
            .unwrap();
        Ok(())
    }
}
