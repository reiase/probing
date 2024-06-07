use argh::FromArgs;
use nix::{sys::signal, unistd::Pid};
use ptrace_inject::{Injector, Process};
use std::{fmt::Error, fs};

/// Probe CLI - A performance and stability diagnostic tool for AI applications
#[derive(FromArgs)]
struct Cli {
    /// DLL file to be injected into the target process (e.g., <location of probe cli>/libprobe.so)
    #[argh(option, short = 'd')]
    dll: Option<std::path::PathBuf>,

    /// target process ID (e.g., 1234)
    #[argh(positional)]
    pid: u32,

    #[argh(subcommand)]
    command: Option<Commands>,
}

/// probe subcmds
#[derive(FromArgs)]
#[argh(subcommand)]
enum Commands {
    Inject(InjectCommand),
    Dump(DumpCommand),
    Pause(PauseCommand),
    Pprof(PprofCommand),
    CatchCrash(CatchCrashCommand),
    ListenRemote(ListenRemoteCommand),
    Execute(ExecuteCommand),
}

/// Inject into target process
#[derive(FromArgs)]
#[argh(subcommand, name = "inject")]
struct InjectCommand {
    /// enable profiling
    #[argh(switch, short = 'P')]
    pprof: bool,

    /// enable handling target process crash
    #[argh(switch, short = 'c')]
    crash: bool,

    /// enable background server
    #[argh(switch, short = 'b')]
    background: bool,

    /// address for remote connection (e.g., 127.0.0.1:8080)
    #[argh(option, short = 'a')]
    address: Option<String>,
}

/// Dump the calling stack of the target process
#[derive(FromArgs)]
#[argh(subcommand, name = "dump")]
struct DumpCommand {}

/// Pause the target process and listen for remote connection
#[derive(FromArgs)]
#[argh(subcommand, name = "pause")]
struct PauseCommand {
    /// address to listen
    #[argh(option, short = 'a')]
    address: Option<String>,
}

/// Start profiling
#[derive(FromArgs)]
#[argh(subcommand, name = "pprof")]
struct PprofCommand {}

/// Handle target process crash
#[derive(FromArgs)]
#[argh(subcommand, name = "catch")]
struct CatchCrashCommand {}

/// Start background server and listen for remote connections
#[derive(FromArgs)]
#[argh(subcommand, name = "listen")]
struct ListenRemoteCommand {
    /// address to listen
    #[argh(positional)]
    address: Option<String>,
}

/// Execute a script in the target process
#[derive(FromArgs)]
#[argh(subcommand, name = "exec")]
struct ExecuteCommand {
    /// script to execute (e.g., /path/to/script.py)
    #[argh(positional)]
    script: String,
}

pub fn main() -> Result<(), Error> {
    let cli: Cli = argh::from_env();
    let pid = Pid::from_raw(cli.pid as i32);

    let mut cmdstr = "".to_string();

    if let Some(cmd) = cli.command {
        let usr1_handler = |cmdstr: String| {
            let process = Process::get(cli.pid).unwrap();
            Injector::attach(process)
                .unwrap()
                .setenv(Some("PROBE_ARGS"), Some(cmdstr.as_str()))
                .unwrap();
            signal::kill(pid, signal::Signal::SIGUSR1).unwrap();
            Ok::<(), Error>(())
        };
        match cmd {
            Commands::Inject(InjectCommand {
                pprof,
                crash,
                background,
                address,
            }) => {
                if pprof {
                    cmdstr.push_str(" -P");
                }
                if crash {
                    cmdstr.push_str(" -c");
                }
                if background {
                    cmdstr.push_str(" -b");
                }
                if let Some(addr) = address {
                    cmdstr.push_str(" -a ");
                    cmdstr.push_str(addr.as_str());
                }
            }
            Commands::Dump(_) => {
                signal::kill(pid, signal::Signal::SIGUSR2).unwrap();
                return Ok(());
            }
            Commands::Pause(PauseCommand { address }) => {
                let cmdstr = if let Some(addr) = address {
                    format!(" -p -a {}", addr)
                } else {
                    " -p".to_string()
                };
                return usr1_handler(cmdstr);
            }
            Commands::Pprof(_) => {
                signal::kill(pid, signal::Signal::SIGPROF).unwrap();
                return Ok(());
            }
            Commands::CatchCrash(_) => todo!(),
            Commands::ListenRemote(ListenRemoteCommand { address }) => {
                let cmdstr = if let Some(addr) = address {
                    format!(" -b -a {}", addr)
                } else {
                    " -b".to_string()
                };
                return usr1_handler(cmdstr);
            }
            Commands::Execute(ExecuteCommand { script }) => {
                return usr1_handler(format!(" -e {}", script));
            }
        }
    }

    let process = Process::get(cli.pid).unwrap();

    let soname = if let Some(path) = cli.dll {
        Some(path)
    } else if let Ok(_path) = fs::read_link("/proc/self/exe") {
        println!(
            "base path: {} : {}",
            _path.display(),
            _path.parent().unwrap().display()
        );
        _path.with_file_name("libprobe.so").into()
    } else {
        None
    };
    println!(
        "Injecting {} into process {} with arguments `{}`",
        soname.clone().unwrap().to_str().unwrap(),
        cli.pid,
        cmdstr,
    );
    Injector::attach(process)
        .unwrap()
        .inject(&soname.unwrap(), Some(cmdstr.as_str()))
        .unwrap();
    Ok(())
}
