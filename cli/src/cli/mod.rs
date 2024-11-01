use anyhow::Result;
use clap::Parser;
use dpp::cli::CtrlSignal;

pub mod commands;
pub mod ctrl;
pub mod inject;
pub mod panel;
pub mod repl;

use crate::cli::ctrl::CtrlChannel;
use commands::Commands;

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
            // Some(Commands::Debug(cmd)) => cmd.run(ctrl),
            // Some(Commands::Performance(cmd)) => cmd.run(ctrl),
            Some(Commands::Panel) => panel::panel_main(ctrl),
            Some(Commands::Repl(cmd)) => cmd.run(ctrl),

            Some(Commands::Enable(feature)) => {
                ctrl::handle(ctrl, CtrlSignal::Enable(feature.clone()))
            }
            Some(Commands::Disable(feature)) => {
                ctrl::handle(ctrl, CtrlSignal::Disable(feature.clone()))
            }
            Some(Commands::Show(topic)) => ctrl::handle(ctrl, CtrlSignal::Show(topic.clone())),
            Some(Commands::Backtrace(cmd)) => {
                ctrl::handle(ctrl, CtrlSignal::Backtrace(cmd.clone()))
            }
            Some(Commands::Trace(cmd)) => ctrl::handle(ctrl, CtrlSignal::Trace(cmd.clone())),
            Some(Commands::Eval { code }) => {
                ctrl::handle(ctrl, CtrlSignal::Eval { code: code.clone() })
            }

            None => {
                let _ = inject::InjectCommand::default().run(ctrl.clone());
                panel::panel_main(ctrl)
            }
        }
    }
}

pub fn run() -> Result<()> {
    let cli: Cli = Cli::parse();

    cli.run()
}
