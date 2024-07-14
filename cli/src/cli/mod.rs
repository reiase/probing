use anyhow::Result;
use clap::Parser;
use nix::{sys::signal, unistd::Pid};

pub mod commands;
pub mod debug;
pub mod inject;
pub mod misc;
pub mod panel;
pub mod performance;
pub mod repl;

mod ctrl;

use hyperparameter::*;
use repl::Repl;

use crate::inject::{Injector, Process};
use commands::{Commands, ReplCommands};

/// Probing CLI - A performance and stability diagnostic tool for AI applications
#[derive(Parser, Debug)]
pub struct Cli {
    /// DLL file to be injected into the target process (e.g., <location of probing cli>/libprobing.so)
    #[arg(short, long, hide = true)]
    dll: Option<std::path::PathBuf>,

    // /// target process ID (e.g., 1234)
    // #[arg(short, long, conflicts_with_all=["name"])]
    // pub pid: Option<i32>,
    /// Send ctrl commands via ptrace
    #[arg(long)]
    ptrace: bool,

    // /// target process name (e.g., "chrome.exe")
    // #[arg(short, long, conflicts_with_all=["pid"])]
    // pub name: Option<String>,
    /// target process, PID (e.g., 1234) or `Name` (e.g., "chrome.exe") for local process, and <ip>:<port> for remote process
    #[arg()]
    target: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        let pid = self.resolve_pid()?;
        let ctrl = if self.ptrace {
            "ptrace".to_string()
        } else {
            "socket".to_string()
        };

        with_params! {
            set probing.cli.ctrl_channel = ctrl;

            self.execute_command(pid)
        }
    }

    fn execute_command(&self, pid: i32) -> Result<()> {
        match &self.command {
            Some(Commands::Inject(cmd)) => cmd.run(pid, &self.dll),
            Some(Commands::Debug(cmd)) => cmd.run(pid),
            Some(Commands::Performance(cmd)) => cmd.run(pid),
            Some(Commands::Misc(cmd)) => cmd.run(pid),
            Some(Commands::Panel) => panel::panel_main(pid),
            Some(Commands::Repl) => {
                let mut repl = Repl::<ReplCommands>::default();
                loop {
                    let line = repl.read_command(">>");
                    println!("== {:?}", line);
                }
            }
            None => {
                let _ = inject::InjectCommand::default().run(pid, &self.dll);
                panel::panel_main(pid)
            }
        }
    }

    fn resolve_pid(&self) -> Result<i32> {
        if let Ok(pid) = self.target.parse::<i32>() {
            return Ok(pid);
        }
        if let [_, _] = self.target.split(":").collect::<Vec<_>>()[..] {
            return Ok(0);
        }

        let pid = Process::by_cmdline(&self.target).map_err(|err| {
            anyhow::anyhow!(
                "failed to find process with cmdline pattern {}: {}",
                self.target,
                err
            )
        })?;
        if let Some(pid) = pid {
            return Ok(pid);
        } else {
            return Err(anyhow::anyhow!("either `pid` or `name` must be specified"));
        }
    }
}

fn send_ctrl(argstr: String, pid: i32) -> Result<()> {
    with_params! {
        get ctrl_channel = probing.cli.ctrl_channel or "socket".to_string();

        match ctrl_channel.as_str() {
            "ptrace" => {send_ctrl_via_ptrace(argstr, pid)},
            _ => {send_ctrl_via_socket(argstr, pid)}
        }
    }
}

fn send_ctrl_via_socket(argstr: String, pid: i32) -> Result<()> {
    eprintln!("sending ctrl commands via unix socket...");
    let argstr = if argstr.starts_with("[") {
        argstr
    } else {
        format!("[{}]", argstr)
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(ctrl::request(pid, "/ctrl", argstr.into()))?;

    Ok(())
}

fn send_ctrl_via_ptrace(argstr: String, pid: i32) -> Result<()> {
    eprintln!("sending ctrl commands via ptrace...");
    let process = Process::get(pid as u32).unwrap();
    Injector::attach(process)
        .unwrap()
        .setenv(Some("PROBING_ARGS"), Some(argstr.as_str()))
        .map_err(|e| anyhow::anyhow!(e))?;
    signal::kill(Pid::from_raw(pid), signal::Signal::SIGUSR1)?;
    Ok(())
}

pub fn run() -> Result<()> {
    let cli: Cli = Cli::parse();

    cli.run()
}
