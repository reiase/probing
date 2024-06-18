pub mod catch;
pub mod commands;
pub mod dump;
pub mod execute;
pub mod inject;
pub mod listen;
pub mod pause;
pub mod perf;

use commands::Commands;

use crate::inject::{Injector, Process};
use anyhow::Context;
use anyhow::Result;
use nix::{sys::signal, unistd::Pid};

use clap::Parser;

/// Probe CLI - A performance and stability diagnostic tool for AI applications
#[derive(Parser)]
pub struct Cli {
    /// DLL file to be injected into the target process (e.g., <location of probe cli>/libprobe.so)
    #[arg(short, long)]
    dll: Option<std::path::PathBuf>,

    /// target process ID (e.g., 1234)
    #[arg()]
    pub pid: i32,

    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        match &self.command {
            Some(Commands::Inject(cmd)) => cmd.run(self.pid, &self.dll),
            Some(Commands::Dump(cmd)) => cmd.run(self.pid),
            Some(Commands::Pause(cmd)) => cmd.run(self.pid),
            Some(Commands::Perf(cmd)) => cmd.run(self.pid),
            Some(Commands::CatchCrash(cmd)) => cmd.run(self.pid),
            Some(Commands::ListenRemote(cmd)) => cmd.run(self.pid),
            Some(Commands::Execute(cmd)) => cmd.run(self.pid),
            None => inject::InjectCommand::default().run(self.pid, &self.dll),
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
