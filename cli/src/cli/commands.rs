use argh::FromArgs;

#[derive(FromArgs)]
#[argh(subcommand)]
pub enum Commands {
    Inject(super::inject::InjectCommand),
    Dump(super::dump::DumpCommand),
    Pause(super::pause::PauseCommand),
    Pprof(super::pprof::PprofCommand),
    CatchCrash(super::catch::CatchCrashCommand),
    ListenRemote(super::listen::ListenRemoteCommand),
    Execute(super::execute::ExecuteCommand),
}
