use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(visible_aliases = ["in", "i"])]
    Inject(super::inject::InjectCommand),
    #[command(visible_aliases = ["dbg", "d"])]
    Debug(super::debug::DebugCommand),
    #[command(visible_aliases = ["perf", "p"])]
    Performance(super::performance::PerfCommand),

    /// Console visualizer
    #[command(visible_aliases = ["pnl", "console"])]
    Panel,
    #[command()]
    Repl,
    #[command(hide=true, visible_aliases = ["m"])]
    Misc(super::misc::MiscCommand),
}

#[derive(Parser, Debug)]
#[command(name = "")]
pub enum ReplCommands {
    #[command(visible_aliases = ["in", "i"])]
    Inject(super::inject::InjectCommand),
    #[command(visible_aliases = ["dbg", "d"])]
    Debug(super::debug::DebugCommand),
    #[command(visible_aliases = ["perf", "p"])]
    Performance(super::performance::PerfCommand),
    #[command(visible_aliases = ["pnl", "console"])]
    Panel,
    #[command()]
    Repl,
    #[command(hide=true, visible_aliases = ["m"])]
    Misc(super::misc::MiscCommand),
}
