use clap::Subcommand;
use probing_common::cli::{BackTraceCommand, Features, ShowCommand};

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(visible_aliases = ["inj", "i"])]
    Inject(super::inject::InjectCommand),
    // #[command(visible_aliases = ["dbg", "d"])]
    // Debug(super::debug::DebugCommand),
    // #[command(visible_aliases = ["perf", "p"])]
    // Performance(super::performance::PerfCommand),

    /// Console visualizer
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

    /// Evaluate code in the target process
    #[command()]
    Eval {
        #[arg()]
        code: String,
    },
}
