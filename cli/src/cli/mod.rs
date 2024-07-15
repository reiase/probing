use anyhow::Result;
use clap::Parser;
use nix::{sys::signal, unistd::Pid};

pub mod commands;
pub mod ctrl;
pub mod debug;
pub mod inject;
pub mod misc;
pub mod panel;
pub mod performance;
pub mod repl;

use hyperparameter::*;
use repl::Repl;

use crate::cli::ctrl::CtrlChannel;
use crate::inject::{Injector, Process};
use commands::{Commands, ReplCommands};

/// Probing CLI - A performance and stability diagnostic tool for AI applications
#[derive(Parser, Debug)]
pub struct Cli {
    /// DLL file to be injected into the target process (e.g., <location of probing cli>/libprobing.so)
    #[arg(short, long, hide = true)]
    dll: Option<std::path::PathBuf>,

    /// Send ctrl commands via ptrace
    #[arg(long)]
    ptrace: bool,

    /// target process, PID (e.g., 1234) or `Name` (e.g., "chrome.exe") for local process, and <ip>:<port> for remote process
    #[arg()]
    target: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        let ctrl: CtrlChannel = self.target.as_str().try_into()?;

        self.execute_command(ctrl)
    }

    fn execute_command(&self, ctrl: CtrlChannel) -> Result<()> {
        match &self.command {
            Some(Commands::Inject(cmd)) => cmd.run(ctrl),
            Some(Commands::Debug(cmd)) => cmd.run(ctrl),
            Some(Commands::Performance(cmd)) => cmd.run(ctrl),
            Some(Commands::Misc(cmd)) => cmd.run(ctrl),
            Some(Commands::Panel) => panel::panel_main(ctrl),
            Some(Commands::Repl) => {
                let mut repl = Repl::<ReplCommands>::default();
                loop {
                    let line = repl.read_command(">>");
                    println!("== {:?}", line);
                }
            }
            None => {
                let _ = inject::InjectCommand::default().run(ctrl.clone());
                panel::panel_main(ctrl)
            }
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
