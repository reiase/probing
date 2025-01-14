use anyhow::{anyhow, Error, Result};
use clap::Args;

use crate::cli::ctrl::CtrlChannel;
use crate::inject::{Injector, Process};
use probing_proto::cli::{CtrlSignal, Features};

/// Inject into the target process
#[derive(Args, Default, Debug)]
pub struct InjectCommand {
    /// enable profiling
    #[arg(short = 'P', long)]
    pprof: bool,

    /// enable handling target process crash
    #[arg(short = 'c', long)]
    crash: bool,

    /// listen for remote connection (e.g., 127.0.0.1:8080)
    #[arg(short = 'l', long, name = "address")]
    listen: Option<String>,
}

impl InjectCommand {
    fn check_library(&self, pid: i32, lib_name: &str) -> Result<bool> {
        Ok(procfs::process::Process::new(pid)?.maps()?.iter().any(|m| {
            matches!(&m.pathname,
                procfs::process::MMapPath::Path(p) if p
                    .file_name()
                    .map(|n| n.to_string_lossy().contains(lib_name))
                    .unwrap_or(false)
            )
        }))
    }

    fn wait_for_library(&self, pid: i32, lib_name: &str) -> Result<()> {
        for _ in 0..15 {
            if self.check_library(pid, lib_name).map_err(|err| {
                eprintln!("Failed to check library: {}", err);
                false
            }).unwrap_or(false) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Err(anyhow!("Library {} not found in target process", lib_name))
    }

    fn build_command_string(&self) -> Result<String> {
        let cmds = [
            self.pprof.then_some(Features::Pprof),
            self.crash.then(|| Features::CatchCrash {
                address: self.listen.clone(),
            }),
            self.listen.as_ref().map(|address| Features::Remote {
                address: Some(address.clone()),
            }),
        ]
        .iter()
        .flatten()
        .map(|x| CtrlSignal::Enable(x.clone()))
        .collect::<Vec<_>>();

        if cmds.is_empty() {
            return Err(anyhow!("No commands to inject"));
        }
        Ok(ron::to_string(&cmds)?)
    }

    fn inject(&self, pid: i32) -> Result<()> {
        let soname = std::fs::read_link("/proc/self/exe")?.with_file_name("libprobing.so");
        let cmd = self.build_command_string().ok();

        println!("Injecting {} into {}", soname.display(), pid);
        Injector::attach(Process::get(pid as u32).map_err(Error::msg)?)
            .map_err(Error::msg)?
            .inject(&soname, cmd.as_deref())
            .map_err(|e| anyhow!("Failed to inject probing: {}", e))
    }

    pub fn run(&self, ctrl: CtrlChannel) -> Result<()> {
        match ctrl {
            CtrlChannel::Ptrace { pid } | CtrlChannel::Local { pid } => {
                if !self.check_library(pid, "libprobing.so")? {
                    self.wait_for_library(pid, "python")?;
                    self.inject(pid)
                } else {
                    ctrl.signal(self.build_command_string()?)
                }
            }
            _ => Ok(()),
        }
    }
}
