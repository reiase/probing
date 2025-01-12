mod asset;
mod report;
mod server;
mod vars;

pub use self::report::start_report_worker;
pub use self::server::start_local;
pub use self::server::start_remote;

pub fn cleanup() -> anyhow::Result<()> {
    let prefix = std::env::var("PROBING_CTRL_ROOT").unwrap_or("/tmp/probing/".to_string());

    let pid = std::process::id();
    let path = format!("{}/{}", prefix, pid);
    let path = std::path::Path::new(&path);
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    Ok(())
}
