use anyhow::Result;
use serde_json::json;

use ppp::cli::ShowCommand;

use crate::core::pltffi;
use crate::core::trace;
use crate::repl::PythonRepl;

pub fn pyhandle(path: &str, query: Option<String>) -> Result<String> {
    let request = format!(
        "handle(path=\"{path}\", query={})\n",
        query
            .map(|qs| { format!("\"{}\"", qs) })
            .unwrap_or("None".to_string())
    );
    let mut repl = PythonRepl::default();
    repl.process(request.as_str())
        .ok_or(anyhow::anyhow!("no result"))
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
        ShowCommand::Objects => pyhandle("/objects", None),
        ShowCommand::Tensors => pyhandle("/torch/tensors", None),
        ShowCommand::Modules => pyhandle("/torch/modules", None),
        ShowCommand::Traceable { filter } => trace::show_traceable(filter),
        ShowCommand::PLT => pltffi::read_plt(),
        ShowCommand::FFI { name } => pltffi::call_func(name),
    }
}
