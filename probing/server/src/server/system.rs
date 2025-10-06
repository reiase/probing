use anyhow::Result;
use probing_proto::prelude::*;

use super::error::ApiResult;

/// Get system overview information about the current process
pub fn get_overview() -> Result<Process> {
    let myself = std::process::id() as i32;

    #[cfg(target_os = "linux")]
    let threads = {
        let current = procfs::process::Process::new(myself)?;
        current
            .tasks()
            .map(|iter| iter.map(|r| r.map(|p| p.tid as u64).unwrap_or(0)).collect())
            .unwrap_or_default()
    };

    #[cfg(target_os = "macos")]
    let threads = vec![];

    let info = Process {
        pid: myself,
        exe: std::env::current_exe()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .to_string(),
        env: std::env::vars().collect(),
        cmd: std::env::args().collect::<Vec<String>>().join(" "),
        cwd: std::env::current_dir()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default()
            .to_string(),
        main_thread: myself as u64,
        threads: threads,
    };
    Ok(info)
}

/// Get system overview information as JSON for API
pub async fn get_overview_json() -> ApiResult<axum::Json<Process>> {
    let overview = get_overview()?;
    Ok(axum::Json(overview))
}
