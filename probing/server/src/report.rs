use std::time::Duration;

use anyhow::Result;

use super::vars::PROBING_ADDRESS;
use crate::server::SERVER_RUNTIME;
use probing_proto::prelude::Node;

pub fn get_hostname() -> Result<String> {
    let uname = nix::sys::utsname::uname()?;
    let hostname = uname.nodename().to_string_lossy().to_string();
    Ok(hostname)
}

pub fn start_report_worker(report_addr: String, local_addr: String) {
    log::debug!("start report worker: {} => {}", local_addr, report_addr);
    SERVER_RUNTIME.spawn(report_worker(report_addr, local_addr));
}

async fn report_worker(report_addr: String, local_addr: String) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

    loop {
        interval.tick().await;

        let report_addr = format!("http://{}/apis/nodes", report_addr);
        let hostname = get_hostname().unwrap_or("localhost".to_string());
        let mut address = local_addr.clone();
        {
            let probing_address = PROBING_ADDRESS.read().unwrap();
            let probing_address: String = (*probing_address).clone();
            if !probing_address.is_empty() {
                address = probing_address;
            }
        }
        let node = Node {
            host: hostname.clone(),
            addr: address,
            local_rank: get_i32_env("LOCAL_RANK"),
            rank: get_i32_env("RANK"),
            world_size: get_i32_env("WORLD_SIZE"),
            group_rank: get_i32_env("GROUP_RANK"),
            group_world_size: get_i32_env("GROUP_WORLD_SIZE"),
            role_name: std::env::var("ROLE_NAME").ok(),
            role_rank: get_i32_env("ROLE_RANK"),
            role_world_size: get_i32_env("ROLE_WORLD_SIZE"),
            status: Some("running".to_string()),
            timestamp: 0,
        };

        log::debug!("reporting node status to {report_addr}: {:?}", node);
        if node.rank == Some(0) {
            probing_core::core::cluster::update_node(node.clone());
        } else {
            match request_remote(&report_addr, node.clone()).await {
                Ok(reply) => {
                    log::debug!("node status reported to {report_addr}: {:?}", reply);
                }
                Err(err) => {
                    log::error!("failed to report {node} to {report_addr}, {err}");
                }
            }
        }
    }
}

fn get_i32_env(name: &str) -> Option<i32> {
    std::env::var(name).unwrap_or_default().parse().ok()
}

async fn request_remote(url: &str, node: Node) -> Result<String> {
    Ok(ureq::put(url)
        .config()
        .no_delay(true)
        .timeout_global(Some(Duration::from_millis(100)))
        .build()
        .send_json(node)?
        .body_mut()
        .read_to_string()?)
}
