use anyhow::Result;
use probing_proto::prelude::*;

use super::error::ApiResult;

/// Get system overview information about the current process
pub fn get_overview() -> Result<Process> {
    let current = procfs::process::Process::myself()?;
    let info = Process {
        pid: current.pid(),
        exe: current
            .exe()
            .map(|exe| exe.to_string_lossy().to_string())
            .unwrap_or("nil".to_string()),
        env: current
            .environ()
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| {
                        let key = k.to_string_lossy().to_string();
                        // Filter out sensitive auth tokens
                        if key.contains("AUTH_TOKEN") {
                            None
                        } else {
                            Some((key, v.to_string_lossy().to_string()))
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
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
    Ok(info)
}

/// Get system overview information as JSON for API
pub async fn get_overview_json() -> ApiResult<axum::Json<Process>> {
    let overview = get_overview()?;
    Ok(axum::Json(overview))
}
