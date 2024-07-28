use clap::Subcommand;
use probing_ppp::cli::{BackTraceCommand, Features, ShowCommand, TraceCommand};

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(visible_aliases = ["inj", "i"])]
    Inject(super::inject::InjectCommand),

    /// Interactive visualizer in terminal
    #[command(visible_aliases = ["pnl", "console"])]
    Panel,

    /// Repl debugging shell
    #[command()]
    Repl(super::repl::ReplCommand),

    /// Enable features (`-h, --help` to see full feature list)
    #[command(subcommand)]
    Enable(Features),

    /// Disable features (see -h, --help above)
    #[command(subcommand)]
    Disable(Features),

    /// Display informations from the target process (see -h, --help above)
    #[command(subcommand)]
    Show(ShowCommand),

    /// Show the backtrace of the target process or thread
    #[command(subcommand, visible_aliases = ["bt"])]
    Backtrace(BackTraceCommand),

    /// Trace function call and variable changes
    #[command(subcommand)]
    Trace(TraceCommand),

    /// Evaluate code in the target process
    #[command()]
    Eval {
        #[arg()]
        code: String,
    },
}
