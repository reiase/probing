use std::ffi::CString;

use anyhow::Result;
use nix::libc::c_char;
use plt_rs::{collect_modules, DynamicLibrary, RelocationTable};
use serde_json::json;

use crate::ctrl::{StringBuilder, StringBuilderAppend};
use crate::repl::PythonRepl;
use probing_ppp::cli::ShowCommand;

type QueryFn = fn() -> *mut c_char;

pub fn pyhandle(path: &str, query: Option<String>) -> String {
    let request = format!(
        "handle(path=\"{path}\", query={})\n",
        query
            .map(|qs| { format!("\"{}\"", qs) })
            .unwrap_or("None".to_string())
    );
    let mut repl = PythonRepl::default();
    repl.process(request.as_str()).unwrap_or("".to_string())
}

pub fn handle(topic: ShowCommand) -> Result<String> {
    match topic {
        ShowCommand::Memory => {
            let current = procfs::process::Process::myself().unwrap();
            let status = current.status()?;
            let memory_info = json!({
                "VmRSS": status.vmrss,
                "VmHWM": status.vmhwm,
                "VmPeak": status.vmpeak,
                "VmPin": status.vmpin,
                "VmLck": status.vmlck,
            });
            Ok(memory_info.to_string())
        }
        ShowCommand::Threads => {
            let current = procfs::process::Process::myself().unwrap();
            let tasks: Vec<u64> = current
                .tasks()
                .map(|iter| iter.map(|r| r.map(|p| p.tid as u64).unwrap_or(0)).collect())
                .unwrap_or_default();
            Ok(serde_json::to_string(&tasks)?)
        }
        ShowCommand::Objects => Ok(pyhandle("/objects", None)),
        ShowCommand::Tensors => Ok(pyhandle("/torch/tensors", None)),
        ShowCommand::Modules => Ok(pyhandle("/torch/modules", None)),
        ShowCommand::PLT => read_plt(),
        ShowCommand::FFI { name } => {
            let answer = unsafe {
                let handle = nix::libc::dlopen(std::ptr::null::<i8>(), nix::libc::RTLD_LAZY);
                if handle.is_null() {
                    eprintln!("unable of open dll");
                    return Err(anyhow::anyhow!("unable of open dll"));
                }
                let symbol = nix::libc::dlsym(handle, name.as_ptr() as *const i8);
                if symbol.is_null() {
                    eprintln!("symbol not found: {}", name);
                    return Err(anyhow::anyhow!("symbol not found: {}", name));
                }
                let symbol: QueryFn = std::mem::transmute(symbol);
                let ptr = symbol();
                let answer = CString::from_raw(ptr)
                    .to_owned()
                    .to_string_lossy()
                    .to_string();
                nix::libc::dlclose(handle);
                answer
            };
            Ok(answer)
        }
    }
}

pub fn read_plt() -> Result<String> {
    let entries = collect_modules();
    let builder: &mut StringBuilder = &mut Default::default();

    for entry in entries.into_iter() {
        format!("[{:?}] Addr: {:#X?}", entry.name(), entry.addr()).append_line(builder);
        if let Ok(dynamic_lib) = DynamicLibrary::initialize(entry) {
            format!(
                "Dynamic String Table Length: {}",
                dynamic_lib.string_table().total_size()
            )
            .append_line(builder);

            let dynamic_symbols = dynamic_lib.symbols().expect("symbols...");
            let string_table = dynamic_lib.string_table();

            "dynamic addend relocations:".append_line(builder);
            if let Some(dyn_relas) = dynamic_lib.addend_relocs() {
                dyn_relas
                    .entries()
                    .iter()
                    .flat_map(|e| {
                        dynamic_symbols.resolve_name(e.symbol_index() as usize, string_table)
                    })
                    .filter(|s| !s.is_empty())
                    .for_each(|s| format!("\t{}", s).append_line(builder));
            }

            "dynamic relocations:".append_line(builder);
            if let Some(dyn_relocs) = dynamic_lib.relocs() {
                dyn_relocs
                    .entries()
                    .iter()
                    .flat_map(|e| {
                        dynamic_symbols.resolve_name(e.symbol_index() as usize, string_table)
                    })
                    .filter(|s| !s.is_empty())
                    .for_each(|s| format!("\t{}", s).append_line(builder));
            }

            "plt:".append_line(builder);
            if let Some(plt) = dynamic_lib.plt() {
                match plt {
                    RelocationTable::WithAddend(rel) => {
                        rel.entries()
                            .iter()
                            .flat_map(|e| {
                                dynamic_symbols
                                    .resolve_name(e.symbol_index() as usize, string_table)
                            })
                            .filter(|s| !s.is_empty())
                            .for_each(|s| format!("\t{}", s).append_line(builder));
                    }
                    RelocationTable::WithoutAddend(rel) => {
                        rel.entries()
                            .iter()
                            .flat_map(|e| {
                                dynamic_symbols
                                    .resolve_name(e.symbol_index() as usize, string_table)
                            })
                            .filter(|s| !s.is_empty())
                            .for_each(|s| format!("\t{}", s).append_line(builder));
                    }
                }
            }
        }
        "".append_line(builder);
    }

    Ok(builder.to_string())
}
