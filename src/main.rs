use clap::{error::Result, Error, Parser, Subcommand};
use nix::{sys::signal, unistd::Pid};
use ptrace_inject::{Injector, Process};
use std::fs;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// dll file to be injected into the target process, default: <location of probe cli>/libprobe.so
    #[arg(long)]
    dll: Option<std::path::PathBuf>,
    /// target process
    pid: u32,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// inject into target process
    #[command(aliases=["i", "in", "ins"])]
    Inject {
        /// enable profiling
        #[arg(short = 'P', long, action)]
        pprof: bool,

        /// enable handling target process crash
        #[arg(short, long, action)]
        crash: bool,

        /// enable background server
        #[arg(short, long, action)]
        background: bool,

        /// address used for listening remote connection
        #[arg(short, long)]
        address: Option<String>,
    },

    /// dump the calling stack of the target process
    #[command(aliases=["d", "du"])]
    Dump {},

    /// pause the target process and listen for remote connection
    #[command(aliases=["p", "pa"])]
    Pause {
        /// address to listen
        address: Option<String>,
    },

    /// start profiling
    #[command(aliases=["pp"])]
    Pprof {},

    /// handle target process crash
    #[command(aliases=["cc"])]
    CatchCrash {},

    /// start background server and listen for remote connections
    #[command(aliases=["l", "listen"])]
    ListenRemote {
        /// address to listen
        address: Option<String>,
    },

    /// execute a script in the target process
    #[command(aliases=["e", "exec"])]
    Execute {
        /// script to execute
        script: String,
    },
}

pub fn main() -> Result<()> {
    let cli = Cli::parse();
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
            Commands::Inject {
                pprof,
                crash,
                background,
                address,
            } => {
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
            Commands::Dump {} => {
                signal::kill(pid, signal::Signal::SIGUSR2).unwrap();
                return Ok(());
            }
            Commands::Pause { address } => {
                let cmdstr = if let Some(addr) = address {
                    format!(" -p -a {}", addr)
                } else {
                    format!(" -p")
                };
                return usr1_handler(cmdstr);
            }
            Commands::Pprof {} => {
                signal::kill(pid, signal::Signal::SIGPROF).unwrap();
                return Ok(());
            }
            Commands::CatchCrash {} => todo!(),
            Commands::ListenRemote { address } => {
                let cmdstr = if let Some(addr) = address {
                    format!(" -b -a {}", addr)
                } else {
                    format!(" -b")
                };
                return usr1_handler(cmdstr);
            }
            Commands::Execute { script } => {
                return usr1_handler(format!(" -e {}", script));
            }
        }
    }

    let process = Process::get(cli.pid).unwrap();

    let soname = if let Some(path) = cli.dll {
        Some(path)
    } else {
        if let Ok(_path) = fs::read_link("/proc/self/exe") {
            println!(
                "base path: {} : {}",
                _path.display(),
                _path.parent().unwrap().display()
            );
            _path.with_file_name("libprobe.so").into()
        } else {
            None
        }
    };
    println!(
        "inject {} into {} with `{}`",
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
