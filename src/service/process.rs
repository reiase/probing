use std::{env, fs, sync::Mutex};

use nix::{
    sys::signal,
    unistd::{sleep, Pid},
};
use ppp::{cli::CtrlSignal, Process};
use procfs::process;

pub fn overview() -> String {
    let current = procfs::process::Process::myself().unwrap();
    let process_info = Process {
        pid: current.pid(),
        exe: current
            .exe()
            .map(|exe| exe.to_string_lossy().to_string())
            .unwrap_or("nil".to_string()),
        env: current
            .environ()
            .map(|m| {
                let envs: Vec<String> = m
                    .iter()
                    .map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy()))
                    .collect();
                envs.join("\n")
            })
            .unwrap_or("".to_string()),
        cmd: current
            .cmdline()
            .map(|cmds| cmds.join(" "))
            .unwrap_or("".to_string()),
        cwd: current
            .cwd()
            .map(|cwd| cwd.to_string_lossy().to_string())
            .unwrap_or("".to_string()),
        main_thread: current
            .task_main_thread()
            .map(|p| p.pid as u64)
            .unwrap_or(0),
        threads: current
            .tasks()
            .map(|iter| iter.map(|r| r.map(|p| p.tid as u64).unwrap_or(0)).collect())
            .unwrap_or_default(),
    };
    serde_json::to_string_pretty(&process_info)
        .unwrap_or("{\"error\": \"error encoding process info.\"}".to_string())
}

pub static CALLSTACK: Mutex<Option<String>> = Mutex::new(None);

pub fn callstack(tid: Option<String>) -> String {
    CALLSTACK
        .lock()
        .map(|mut cs| {
            *cs = None;
        })
        .unwrap();
    let cmd = CtrlSignal::Dump;
    let cmd = ron::to_string(&cmd).unwrap_or("[]".to_string());
    env::set_var("PROBING_ARGS", cmd);
    let mut pid = process::Process::myself().unwrap().pid();
    if let Some(tid) = tid {
        if let Ok(tid) = tid.parse::<i32>() {
            pid = tid;
        }
    }
    signal::kill(Pid::from_raw(pid), signal::SIGUSR1).unwrap();
    sleep(1);
    CALLSTACK
        .lock()
        .map(|cs| cs.clone().unwrap_or("no call stack".to_string()))
        .unwrap_or("no call stack".to_string())
}

pub fn files(path: Option<String>) -> String {
    if let Some(path) = path {
        fs::read_to_string(path).unwrap_or_default()
    } else {
        "".to_string()
    }
}
