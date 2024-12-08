use anyhow::Result;
use clap::Parser;
use process_monitor::ProcessMonitor;
use repl::ReplCommand;

pub mod commands;
pub mod ctrl;
pub mod inject;
pub mod process_monitor;
pub mod repl;

use crate::cli::ctrl::CtrlChannel;
use commands::Commands;
use probing_dpp::cli::CtrlSignal as Signal;
use probing_dpp::protocol::query::Query;

/// Probing CLI - A performance and stability diagnostic tool for AI applications
#[derive(Parser, Debug)]
pub struct Cli {
    /// Enable verbose mode
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Send ctrl commands via ptrace
    #[arg(long)]
    ptrace: bool,

    /// target process, PID (e.g., 1234) or `Name` (e.g., "chrome.exe") for local process, and <ip>:<port> for remote process
    #[arg()]
    target: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        let target = self.target.clone().unwrap_or("0".to_string());
        let ctrl: CtrlChannel = target.as_str().try_into()?;

        self.execute_command(ctrl)
    }

    fn execute_command(&self, ctrl: CtrlChannel) -> Result<()> {
        if self.command.is_none() {
            inject::InjectCommand::default().run(ctrl.clone())?;
            ReplCommand::new().run(ctrl)?;
            return Ok(());
        }
        let command = self.command.as_ref().unwrap();
        match command {
            Commands::Inject(cmd) => cmd.run(ctrl),
            Commands::Repl(cmd) => cmd.run(ctrl),

            Commands::Enable(feature) => ctrl::handle(ctrl, Signal::Enable(feature.clone())),
            Commands::Disable(feature) => ctrl::handle(ctrl, Signal::Disable(feature.clone())),
            Commands::Show(topic) => ctrl::handle(ctrl, Signal::Show(topic.clone())),
            Commands::Backtrace(cmd) => ctrl::handle(ctrl, Signal::Backtrace(cmd.clone())),
            Commands::Trace(cmd) => ctrl::handle(ctrl, Signal::Trace(cmd.clone())),
            Commands::Eval { code } => ctrl::handle(ctrl, Signal::Eval { code: code.clone() }),

            Commands::Query { query } => ctrl::query(
                ctrl,
                Query {
                    expr: query.clone(),
                    opts: None,
                },
            ),
            Commands::Launch { recursive, args } => {
                ProcessMonitor::new(args, *recursive)?.monitor()
            }
        }
    }
}

pub fn run() -> Result<()> {
    Cli::parse().run()
}
