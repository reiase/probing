use clap::Subcommand;

use super::store::StoreCommand;

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(visible_aliases = ["in", "i"])]
    Inject(super::inject::InjectCommand),

    /// List all processes with injected probes
    #[command(visible_aliases = ["ls", "l"])]
    List {
        #[arg(short, long, help = "Show detailed information")]
        verbose: bool,

        #[arg(short, long, help = "Show processes as a tree structure")]
        tree: bool,
    },

    /// Display or modify the configuration
    #[command(visible_aliases = ["cfg", "c"])]
    Config { setting: Option<String> },

    /// Show the backtrace of the target process or thread
    #[command(visible_aliases = ["bt", "b"])]
    Backtrace { tid: Option<i32> },

    /// Evaluate Python code in the target process
    #[command(visible_aliases = ["e"])]
    Eval {
        #[arg()]
        code: String,
    },

    /// Query data from the target process
    #[command(visible_aliases = ["q"])]
    Query {
        #[arg()]
        query: String,
    },

    /// Launch new Python process
    #[command()]
    Launch {
        #[arg(short, long)]
        recursive: bool,

        #[arg()]
        args: Vec<String>,
    },

    /// Access various storage backends
    #[command(subcommand = false, hide = true)]
    Store(StoreCommand),
}
