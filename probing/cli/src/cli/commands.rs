use clap::Subcommand;
use probing_proto::cli::{BackTraceCommand, Features, ShowCommand, TraceCommand};

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(visible_aliases = ["inj", "i"])]
    Inject(super::inject::InjectCommand),

    /// Enable features (`-h, --help` to see full feature list)
    #[command()]
    Enable { feature: String },

    /// Disable features (see -h, --help above)
    #[command(subcommand)]
    Disable(Features),

    /// Display or modify the configuration
    #[command()]
    Config {
        setting: Option<String>,
    },

    // /// Display informations from the target process (see -h, --help above)
    // #[command(subcommand)]
    // Show(ShowCommand),

    /// Show the backtrace of the target process or thread
    #[command(subcommand, visible_aliases = ["bt"])]
    Backtrace(BackTraceCommand),

    // /// Trace function call and variable changes
    // #[command(subcommand)]
    // Trace(TraceCommand),

    /// Evaluate code in the target process
    #[command()]
    Eval {
        #[arg()]
        code: String,
    },

    /// Query data from the target process
    #[command()]
    Query {
        #[arg()]
        query: String,
    },

    #[command()]
    Launch {
        #[arg(short, long)]
        recursive: bool,

        #[arg()]
        args: Vec<String>,
    },
}
