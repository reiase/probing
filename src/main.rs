use clap::{error::Result, Error, Parser};
use nix::{sys::signal, unistd::Pid};
use ptrace_inject::{Injector, Process};
use std::fs;

#[derive(Parser, Debug)]
struct DeriveArgs {
    /// dll file to be injected into the target process, default: <location of probe cli>/libprobe.so
    #[arg(long)]
    dll: Option<std::path::PathBuf>,

    /// signal libprobe to dump the calling stack of the target process
    #[arg(short, long, action)]
    dump: bool,

    /// signal libprobe to pause the target process and listen for remote connection
    #[arg(short, long, action)]
    pause: bool,

    /// signal libprobe to start profiling
    #[arg(short = 'P', long, action)]
    pprof: bool,

    /// signal libprobe to handle target process crash
    #[arg(short, long, action)]
    crash: bool,

    /// signal libprobe to start background server
    #[arg(short, long, action)]
    background: bool,

    /// signal libprobe to execute a script in the target process
    #[arg(short, long)]
    pub execute: Option<String>,

    /// address used for listening remote connection
    #[arg(short, long)]
    pub address: Option<String>,

    #[arg(short, long, action)]
    test: bool,

    /// target process
    #[arg()]
    pid: u32,
}

impl DeriveArgs {
    pub fn to_string(&self) -> String {
        let mut ret = "".to_string();
        if self.dump {
            ret.push_str(" -d");
        }
        if self.pause {
            ret.push_str(" -p");
        }
        if self.pprof {
            ret.push_str(" -P");
        }
        if self.crash {
            ret.push_str(" -c");
        }
        if self.background {
            ret.push_str(" -b");
        }
        if let Some(script) = &self.execute {
            ret.push_str(" -e ");
            ret.push_str(script.as_str());
        }
        if let Some(addr) = &self.address {
            ret.push_str(" -a ");
            ret.push_str(addr.as_str());
        }
        ret
    }
}

pub fn main() -> Result<()> {
    let args = DeriveArgs::parse();
    let args_str = args.to_string();

    let pid = Pid::from_raw(args.pid as i32);

    let usr1_handler = || {
        let process = Process::get(args.pid).unwrap();
        Injector::attach(process)
            .unwrap()
            .setenv(Some("PROBE_ARGS"), Some(args_str.as_str()))
            .unwrap();
        signal::kill(pid, signal::Signal::SIGUSR1).unwrap();
        Ok::<(), Error>(())
    };

    if args.dump {
        signal::kill(pid, signal::Signal::SIGUSR2).unwrap();
        return Ok(());
    }

    if args.pause || args.execute.is_some() {
        return usr1_handler();
    }

    if args.pprof {
        signal::kill(pid, signal::Signal::SIGPROF).unwrap();
        return Ok(());
    }

    let process = Process::get(args.pid).unwrap();

    let soname = if let Some(path) = args.dll {
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
        args.pid,
        args_str
    );
    Injector::attach(process)
        .unwrap()
        .inject(&soname.unwrap(), Some(args_str.as_str()))
        .unwrap();
    Ok(())
}
