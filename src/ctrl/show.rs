use anyhow::Result;
use probing_common::cli::ShowCommand;

use super::not_implemented;
use crate::handlers::read_plt;
use crate::repl::PythonRepl;

pub fn pyhandle(path: &str, query: Option<String>) -> String {
    let request = format!(
        "handle(path=\"{}\", query={})\n",
        path,
        query
            .map(|qs| { format!("\"{}\"", qs) })
            .unwrap_or("None".to_string())
    );
    let mut repl = PythonRepl::default();
    repl.process(request.as_str()).unwrap_or("".to_string())
}

pub fn handle(topic: ShowCommand) -> Result<String> {
    match topic {
        ShowCommand::Memory => not_implemented(),
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
    }
}
