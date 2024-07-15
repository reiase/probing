use anyhow::Result;
use clap::Parser;

pub mod commands;
pub mod ctrl;
pub mod debug;
pub mod inject;
pub mod misc;
pub mod panel;
pub mod performance;
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
            Some(Commands::Debug(cmd)) => cmd.run(ctrl),
            Some(Commands::Performance(cmd)) => cmd.run(ctrl),
            Some(Commands::Misc(cmd)) => cmd.run(ctrl),
            Some(Commands::Panel) => panel::panel_main(ctrl),
            Some(Commands::Repl(cmd)) => cmd.run(ctrl),
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
