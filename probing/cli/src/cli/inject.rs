use anyhow::{anyhow, Error, Result};
use clap::Args;

use probing_proto::protocol::query::Query;

use crate::cli::ctrl::ProbeEndpoint;
use crate::inject::{Injector, Process};

use super::ctrl;

/// Inject into the target process
#[derive(Args, Default, Debug)]
pub struct InjectCommand {
    #[arg(short='D', long="define", num_args=1..)]
    settings: Vec<String>,
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
            if self
                .check_library(pid, lib_name)
                .map_err(|err| {
                    eprintln!("Failed to check library: {}", err);
                    false
                })
                .unwrap_or(false)
            {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Err(anyhow!("Library {} not found in target process", lib_name))
    }

    fn build_settings(&self) -> Vec<String> {
        self.settings
            .iter()
            .map(|setting| {
                if setting.starts_with("probing.") {
                    setting.replace(".", "_")
                } else {
                    setting.clone()
                }
            })
            .collect()
    }

    fn inject(&self, pid: i32) -> Result<()> {
        let soname = std::fs::read_link("/proc/self/exe")?.with_file_name("libprobing.so");
        let settings = self.build_settings();

        println!("Injecting {} into {}", soname.display(), pid);
        Injector::attach(Process::get(pid as u32).map_err(Error::msg)?)
            .map_err(Error::msg)?
            .inject(&soname, settings)
            .map_err(|e| anyhow!("Failed to inject probing: {}", e))
    }

    pub fn run(&self, ctrl: ProbeEndpoint) -> Result<()> {
        match ctrl {
            ProbeEndpoint::Ptrace { pid } | ProbeEndpoint::Local { pid } => {
                if !self.check_library(pid, "libprobing.so")? {
                    self.wait_for_library(pid, "python")?;
                    self.inject(pid)
                } else {
                    let settings = self.build_settings();
                    let query: Vec<String> = settings
                        .iter()
                        .map(|setting| format!("set {setting}"))
                        .collect();
                    let query = query.join(";");
                    ctrl::query(
                        ctrl,
                        Query {
                            expr: query,
                            opts: None,
                        },
                    )
                }
            }
            _ => Ok(()),
        }
    }
}
