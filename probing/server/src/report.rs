use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
use std::time::Duration;

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use probing_proto::prelude::Node;

use super::vars::PROBING_ADDRESS;
use crate::server::SERVER_RUNTIME;

static REPORT_INTERVAL: AtomicU64 = AtomicU64::new(10);
static REPORT_TIMEOUT: AtomicU64 = AtomicU64::new(3);
static REPORT_WORKER: Lazy<Mutex<Option<tokio::task::JoinHandle<()>>>> =
    Lazy::new(|| Mutex::new(None));
static REPORT_AGENT: Lazy<Mutex<Option<(u64, ureq::Agent)>>> = Lazy::new(|| Mutex::new(None));

pub fn get_hostname() -> Result<String> {
    let hostname = nix::sys::utsname::uname()?
        .nodename()
        .to_string_lossy()
        .trim()
        .to_string();
    (!hostname.is_empty())
        .then_some(hostname)
        .ok_or_else(|| anyhow!("hostname is empty"))
}

pub fn start_report_worker(report_addr: String, local_addr: String) {
    let mut worker = REPORT_WORKER.lock().unwrap();
    if let Some(handle) = worker.take() {
        handle.abort();
    }
    if report_addr.trim().is_empty() {
        return;
    }
    let url = match normalize_report_url(&report_addr) {
        Ok(url) => url,
        Err(err) => {
            log::warn!("invalid node report address {report_addr:?}: {err}");
            return;
        }
    };
    log::debug!("start report worker: {local_addr} => {url}");
    *worker = Some(SERVER_RUNTIME.spawn(report_worker(url, local_addr)));
}

pub(crate) fn set_report_interval_secs(value: u64) {
    REPORT_INTERVAL.store(value, Ordering::Relaxed);
}

pub(crate) fn set_report_timeout_secs(value: u64) {
    REPORT_TIMEOUT.store(value, Ordering::Relaxed);
}

async fn report_worker(url: String, local_addr: String) {
    let hostname = get_hostname().unwrap_or_else(|err| {
        log::warn!("failed to get hostname: {err}; using localhost");
        "localhost".into()
    });
    let seed = get_i32_env("RANK")
        .map(|value| value as u64)
        .unwrap_or(std::process::id() as u64);
    let mut failures = 0_u32;

    loop {
        let result = match advertised_address(&hostname, &local_addr) {
            Some(addr) => report_once(&url, &hostname, addr).await,
            None => Ok(()),
        };
        match result {
            Ok(()) => {
                if failures > 0 {
                    log::info!("node reporting to {url} recovered after {failures} failure(s)");
                }
                failures = 0;
            }
            Err(err) => {
                failures = failures.saturating_add(1);
                if failures == 1 {
                    log::warn!("failed to report node to {url}: {err}");
                } else {
                    log::debug!("failed to report node to {url} ({failures} failures): {err}");
                }
            }
        }
        let interval = REPORT_INTERVAL.load(Ordering::Relaxed);
        tokio::time::sleep(backoff_delay(interval, failures, seed)).await;
    }
}

async fn report_once(url: &str, hostname: &str, addr: String) -> Result<()> {
    let node = Node {
        host: hostname.into(),
        addr,
        local_rank: get_i32_env("LOCAL_RANK"),
        rank: get_i32_env("RANK"),
        world_size: get_i32_env("WORLD_SIZE"),
        group_rank: get_i32_env("GROUP_RANK"),
        group_world_size: get_i32_env("GROUP_WORLD_SIZE"),
        role_name: get_string_env("ROLE_NAME"),
        role_rank: get_i32_env("ROLE_RANK"),
        role_world_size: get_i32_env("ROLE_WORLD_SIZE"),
        status: Some("running".into()),
        timestamp: 0,
    };
    if node.rank == Some(0) {
        probing_core::core::cluster::update_node(node);
        Ok(())
    } else {
        request_remote(
            url.to_owned(),
            node,
            Duration::from_secs(REPORT_TIMEOUT.load(Ordering::Relaxed)),
        )
        .await
    }
}

fn get_string_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn get_i32_env(name: &str) -> Option<i32> {
    let value = get_string_env(name)?;
    match value.parse() {
        Ok(value) => Some(value),
        Err(err) => {
            log::warn!("ignoring invalid integer environment variable {name}={value:?}: {err}");
            None
        }
    }
}

fn advertised_address(hostname: &str, configured: &str) -> Option<String> {
    let published = PROBING_ADDRESS.read().unwrap();
    let candidate = if published.trim().is_empty() {
        configured
    } else {
        &published
    };
    let candidate = candidate.trim().trim_matches(['\'', '"']);
    match candidate.parse::<std::net::SocketAddr>() {
        Ok(addr) if addr.port() == 0 => None,
        Ok(addr) if addr.ip().is_unspecified() => Some(format!("{hostname}:{}", addr.port())),
        Ok(addr) => Some(addr.to_string()),
        Err(_) if candidate.is_empty() || candidate.ends_with(":0") => None,
        Err(_) => Some(candidate.to_owned()),
    }
}

pub(crate) fn normalize_report_url(addr: &str) -> Result<String> {
    let addr = addr.trim().trim_matches(['\'', '"']);
    if addr.starts_with("https://") {
        return Err(anyhow!("HTTPS is not supported"));
    }
    let raw = if addr.starts_with("http://") {
        addr.to_owned()
    } else if addr.contains("://") {
        return Err(anyhow!("unsupported URL scheme"));
    } else {
        format!("http://{addr}")
    };
    let uri: axum::http::Uri = raw
        .parse()
        .map_err(|err| anyhow!("invalid HTTP address: {err}"))?;
    let authority = uri.authority().ok_or_else(|| anyhow!("host is missing"))?;
    if authority.port_u16().is_none() {
        return Err(anyhow!("port is missing"));
    }
    if !matches!(uri.path(), "" | "/") || uri.query().is_some() {
        return Err(anyhow!("path and query are not allowed"));
    }
    Ok(format!("http://{authority}/apis/nodes"))
}

fn backoff_delay(interval: u64, failures: u32, seed: u64) -> Duration {
    if failures == 0 {
        return Duration::from_secs(interval);
    }
    let multiplier = 1_u64
        .checked_shl(failures.saturating_sub(1).min(6))
        .unwrap_or(64);
    let base = interval.saturating_mul(multiplier).min(60);
    let jitter_ms =
        base * 1_000 * (seed.wrapping_mul(1_103_515_245).wrapping_add(12_345) % 21) / 100;
    Duration::from_millis((base * 1_000 + jitter_ms).min(60_000))
}

fn report_agent(timeout: Duration) -> ureq::Agent {
    let key = timeout.as_secs();
    let mut cached = REPORT_AGENT.lock().unwrap();
    if cached.as_ref().map(|(cached_key, _)| *cached_key) != Some(key) {
        let agent = ureq::Agent::config_builder()
            .no_delay(true)
            .timeout_global(Some(timeout))
            .build()
            .into();
        *cached = Some((key, agent));
    }
    cached.as_ref().unwrap().1.clone()
}
async fn request_remote(url: String, node: Node, timeout: Duration) -> Result<()> {
    let agent = report_agent(timeout);
    tokio::task::spawn_blocking(move || -> Result<()> {
        agent.put(&url).send_json(node)?;
        Ok(())
    })
    .await
    .map_err(|err| anyhow!("node report task failed: {err}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_url_validation() {
        assert_eq!(
            normalize_report_url("127.0.0.1:9922").unwrap(),
            "http://127.0.0.1:9922/apis/nodes"
        );
        assert_eq!(
            normalize_report_url("http://host:80/").unwrap(),
            "http://host:80/apis/nodes"
        );
        assert!(normalize_report_url("https://host:443").is_err());
        assert!(normalize_report_url("http://host").is_err());
        assert!(normalize_report_url("http://host:80/path").is_err());
    }

    #[test]
    fn backoff_is_bounded() {
        assert_eq!(backoff_delay(10, 0, 0), Duration::from_secs(10));
        assert!(
            (Duration::from_secs(10)..=Duration::from_secs(12)).contains(&backoff_delay(10, 1, 1))
        );
        assert_eq!(backoff_delay(10, 20, 1), Duration::from_secs(60));
    }
}
