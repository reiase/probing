use clap::{error::Result, Parser};
use nix::{sys::signal, unistd::Pid};
use ptrace_inject::{Injector, Process};
use std::fs;

#[derive(Parser, Debug)]
struct DeriveArgs {
    #[arg(long)]
    dll: Option<std::path::PathBuf>,

    #[arg(short, long, action)]
    dump: bool,

    #[arg(short, long, action)]
    pause: bool,

    #[arg()]
    pid: u32,
}
pub fn main() -> Result<()> {
    let args = DeriveArgs::parse();

    let pid = Pid::from_raw(args.pid as i32);
    if args.dump {
        signal::kill(pid, signal::Signal::SIGUSR2).unwrap();
        return Ok(());
    }

    if args.pause {
        signal::kill(pid, signal::Signal::SIGUSR1).unwrap();
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
        "inject {} into {}",
        soname.clone().unwrap().to_str().unwrap(),
        args.pid
    );
    Injector::attach(process)
        .unwrap()
        .inject(&soname.unwrap())
        .unwrap();
    Ok(())
}
