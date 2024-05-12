use clap::{error::Result, Parser};
use ptrace_inject::{Injector, Process};
use std::fs;

#[derive(Parser, Debug)]
struct DeriveArgs {
    #[arg(short, long)]
    pid: u32,

    #[arg()]
    dll: Option<std::path::PathBuf>,
}
pub fn main() -> Result<()> {
    let args = DeriveArgs::parse();
    let process = Process::get(args.pid).unwrap();

    let soname = if let Some(path) = args.dll {
        Some(path)
    } else {
        if let Ok(_path) = fs::read_link("/proc/self/exe") {
            println!("base path: {} : {}", _path.display(), _path.parent().unwrap().display());
             _path.with_file_name("libprobe.so").into()
        } else {
            None
        }
    };
    println!("inject {} into {}", soname.clone().unwrap().to_str().unwrap(), args.pid);
    Injector::attach(process)
        .unwrap()
        .inject(&soname.unwrap())
        .unwrap();
    Ok(())
}
