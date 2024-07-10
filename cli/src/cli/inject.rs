use anyhow::Result;
use clap::Args;
use probing_common::cli::ProbingCommand;
use std::fs;

use crate::{
    cli::send_ctrl,
    inject::{Injector, Process},
};

/// Inject into target process
#[derive(Args, Default)]
pub struct InjectCommand {
    /// enable profiling
    #[arg(short = 'P', long)]
    pprof: bool,

    /// enable handling target process crash
    #[arg(short = 'c', long)]
    crash: bool,

    /// listen for remote connection (e.g., 127.0.0.1:8080)
    #[arg(short = 'a', long, name = "address")]
    listen: Option<String>,

    /// execute a script (e.g., /path/to/script.py)
    #[arg(short = 'e', long, name = "script")]
    execute: Option<String>,
}

impl InjectCommand {
    fn has_probing(&self, pid: i32) -> bool {
        let target = procfs::process::Process::new(pid).unwrap();
        let maps = target.maps().unwrap();
        maps.iter()
            .map(|m| match &m.pathname {
                procfs::process::MMapPath::Path(p) => p.to_string_lossy().to_string(),
                _ => "".to_string(),
            })
            .any(|p| p.ends_with("libprobing.so") || p.ends_with("probing.abi3.so"))
    }

    pub fn run(&self, pid: i32, dll: &Option<std::path::PathBuf>) -> Result<()> {
        let mut cmds = vec![];
        if self.pprof {
            cmds.push(ProbingCommand::Perf);
        }
        if self.crash {
            cmds.push(ProbingCommand::CatchCrash);
        }
        if let Some(address) = &self.listen {
            cmds.push(ProbingCommand::ListenRemote {
                address: Some(address.clone()),
            });
        }
        if let Some(script) = &self.execute {
            cmds.push(ProbingCommand::Execute {
                script: script.clone(),
            })
        }
        let soname = if let Some(path) = dll {
            Some(path.clone())
        } else if let Ok(_path) = fs::read_link("/proc/self/exe") {
            println!(
                "base path: {} : {}",
                _path.display(),
                _path.parent().unwrap().display()
            );
            _path.with_file_name("libprobing.so").into()
        } else {
            None
        };

        let argstr = ron::to_string(&cmds)?;
        println!(
            "Injecting {} into process {} with arguments `{}`",
            soname.clone().unwrap().to_str().unwrap(),
            pid,
            argstr,
        );

        let process = Process::get(pid as u32).unwrap();
        if self.has_probing(pid) {
            let argstr = ron::to_string(&cmds)?;
            send_ctrl(argstr, pid)
        } else {
            Injector::attach(process)
                .unwrap()
                .inject(&soname.unwrap(), Some(argstr.as_str()))
                .map_err(|err| anyhow::anyhow!("failed to inject probing to {}: {}", pid, err))
        }
    }
}
