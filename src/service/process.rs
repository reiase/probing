use probe_common::Process;

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
        ..Default::default()
    };
    serde_json::to_string_pretty(&process_info)
        .unwrap_or("{\"error\": \"error encoding process info.\"}".to_string())
}
