pub mod catch;
pub mod commands;
pub mod dump;
pub mod execute;
pub mod inject;
pub mod listen;
pub mod pause;
pub mod pprof;

use commands::Commands;

use anyhow::Result;
use argh::FromArgs;
use nix::{sys::signal, unistd::Pid};
use crate::inject::{Injector, Process};

/// Probe CLI - A performance and stability diagnostic tool for AI applications
#[derive(FromArgs)]
pub struct Cli {
    /// DLL file to be injected into the target process (e.g., <location of probe cli>/libprobe.so)
    #[argh(option, short = 'd')]
    dll: Option<std::path::PathBuf>,

    /// target process ID (e.g., 1234)
    #[argh(positional)]
    pub pid: i32,

    #[argh(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        match &self.command {
            Some(Commands::Inject(cmd)) => cmd.run(self.pid, &self.dll),
            Some(Commands::Dump(cmd)) => cmd.run(self.pid),
            Some(Commands::Pause(cmd)) => cmd.run(self.pid),
            Some(Commands::Pprof(cmd)) => cmd.run(self.pid),
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
        .unwrap();
    signal::kill(Pid::from_raw(pid), signal::Signal::SIGUSR1).unwrap();
    Ok(())
}
