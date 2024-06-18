use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Inject(super::inject::InjectCommand),
    Dump(super::dump::DumpCommand),
    Pause(super::pause::PauseCommand),
    Perf(super::perf::PerfCommand),
    CatchCrash(super::catch::CatchCrashCommand),
    ListenRemote(super::listen::ListenRemoteCommand),
    Execute(super::execute::ExecuteCommand),
}
