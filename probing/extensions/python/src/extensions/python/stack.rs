use probing_proto::prelude::{CallFrame, Value};
use std::collections::HashMap;
use py_spy::stack_trace::LocalVariable;


pub fn get_python_stacks(pid: py_spy::Pid) -> Option<Vec<CallFrame>> {
    let mut frames = vec![];

    // Create a new PythonSpy object with the default config options
    let mut config = py_spy::Config::default();
    config.blocking = py_spy::config::LockingStrategy::NonBlocking;
    let process = py_spy::PythonSpy::new(pid, &config);

    // get stack traces for each thread in the process
    let traces = match process.unwrap().get_stack_traces() {
        Ok(traces) => traces,
        Err(e) => {
            log::error!("Failed to get stack traces: {}", e);
            return None;
        }
    };

    // figure out the python stack for each thread
    for trace in traces {
        log::debug!("!!!!Thread {:#X} ({})", trace.thread_id, trace.status_str());
        for frame in &trace.frames {
            log::debug!("!!!!!\t {} ({}:{})", frame.name, frame.filename, frame.line);
            frames.push(CallFrame::PyFrame {
                file: frame.filename.clone(),
                func: frame.name.clone(),
                lineno: frame.line as i64,
                locals: convert_locals_to_map(frame.locals.clone()),
            });
        }
    }

    Some(frames)
}

pub fn convert_locals_to_map(locals: Option<Vec<LocalVariable>>) -> HashMap<String, Value> {
    log::debug!("Converting locals to map...{:#?}", locals);
    let mut map = HashMap::new();
    
    if let Some(local_vars) = locals {
        for var in local_vars {
            let value = Value {
                id: var.addr as u64,
                class: "Variable".to_string(),
                shape: None, 
                dtype: None, 
                device: None, 
                value: var.repr,
            };
            
            map.insert(var.name, value);
        }
    }
    log::debug!("Converted locals to map: {:?}", map);
    map
}