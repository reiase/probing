use anyhow::Result;
use clap::Args;
use probing_common::cli::ProbingCommand;
use std::fs;

use crate::{
    cli::send_ctrl,
    inject::{Injector, Process},
};

use super::ctrl::CtrlChannel;

/// Inject into target process
#[derive(Args, Default, Debug)]
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

    fn parse_flags(&self) -> String {
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
        ron::to_string(&cmds).unwrap()
    }

    pub fn run(&self, ctrl: CtrlChannel) -> Result<()> {
        let argstr = self.parse_flags();
        let soname =
            fs::read_link("/proc/self/exe").map(|path| path.with_file_name("libprobing.so"))?;

        // send ctrl cmds
        match ctrl {
            CtrlChannel::Ptrace { pid } | CtrlChannel::Local { pid } if self.has_probing(pid) => {
                send_ctrl(argstr, pid)
            }
            CtrlChannel::Ptrace { pid } | CtrlChannel::Local { pid } => {
                let process = Process::get(pid as u32).unwrap();
                println!(
                    "Injecting {} into process {pid} with arguments `{argstr}`",
                    soname.to_str().unwrap(),
                );
                Injector::attach(process)
                    .unwrap()
                    .inject(&soname, Some(argstr.as_str()))
                    .map_err(|err| anyhow::anyhow!("failed to inject probing to {}: {}", pid, err))
            }
            CtrlChannel::Remote { addr } => todo!(),
        }
    }
}
