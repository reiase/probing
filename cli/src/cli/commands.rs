use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    #[command(visible_aliases = ["in", "i"])]
    Inject(super::inject::InjectCommand),
    #[command(visible_aliases = ["dbg", "d"])]
    Debug(super::debug::DebugCommand),
    #[command(visible_aliases = ["perf", "p"])]
    Performance(super::performance::PerfCommand),
    #[command(visible_aliases = ["m"])]
    Misc(super::misc::MiscCommand),
    #[command(visible_aliases = ["con"])]
    Console,
    // CatchCrash(super::catch::CatchCrashCommand),
}
