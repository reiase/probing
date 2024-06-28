use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use nix::{sys::signal, unistd::Pid};

pub mod catch;
pub mod commands;
pub mod debug;
pub mod inject;
pub mod performance;

use crate::inject::{Injector, Process};
use commands::Commands;

/// Probe CLI - A performance and stability diagnostic tool for AI applications
#[derive(Parser)]
pub struct Cli {
    /// DLL file to be injected into the target process (e.g., <location of probe cli>/libprobe.so)
    #[arg(short, long)]
    dll: Option<std::path::PathBuf>,

    /// target process ID (e.g., 1234)
    #[arg(short, long, conflicts_with_all=["name"])]
    pub pid: Option<i32>,

    /// target process name (e.g., "chrome.exe")
    #[arg(short, long, conflicts_with_all=["pid"])]
    pub name: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        let pid = {
            if let Some(pid) = self.pid {
                pid
            } else if let Some(name) = self.name.as_ref() {
                let process = Process::by_name(name.as_str())
                    .map_err(|err| {
                        anyhow::anyhow!("failed to find process with name {}: {}", name, err)
                    })?
                    .unwrap();
                process.pid()
            } else {
                return Err(anyhow::anyhow!("either `pid` or `name` must be specified"));
            }
        };
        match &self.command {
            Some(Commands::Inject(cmd)) => cmd.run(pid, &self.dll),
            Some(Commands::Debug(cmd)) => cmd.run(pid),
            Some(Commands::Performance(cmd)) => cmd.run(pid),
            // Some(Commands::CatchCrash(cmd)) => cmd.run(self.pid),
            None => inject::InjectCommand::default().run(pid, &self.dll),
        }
    }
}

fn usr1_handler(argstr: String, pid: i32) -> Result<()> {
    let process = Process::get(pid as u32).unwrap();
    Injector::attach(process)
        .unwrap()
        .setenv(Some("PROBE_ARGS"), Some(argstr.as_str()))
        .map_err(|e| anyhow::anyhow!(e))
        .context("failed to setup `PROBE_ARGS`")?;
    signal::kill(Pid::from_raw(pid), signal::Signal::SIGUSR1)?;
    Ok(())
}

pub fn run() -> Result<()> {
    let cli: Cli = Cli::parse();

    cli.run()
}
