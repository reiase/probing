use clap::Subcommand;

use super::store::StoreCommand;

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(visible_aliases = ["inj", "i"])]
    Inject(super::inject::InjectCommand),

    /// Display or modify the configuration
    #[command()]
    Config { setting: Option<String> },

    /// Show the backtrace of the target process or thread
    #[command(visible_aliases = ["bt"])]
    Backtrace { tid: Option<i32> },

    /// Evaluate Python code in the target process
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

    /// Launch new Python process
    #[command()]
    Launch {
        #[arg(short, long)]
        recursive: bool,

        #[arg()]
        args: Vec<String>,
    },

    /// Access various storage backends
    #[command(subcommand=false, hide=true)]
    Store(StoreCommand),
}
