use anyhow::Result;
use log::debug;

use probing_proto::prelude::Node;

use super::vars::PROBING_ADDRESS;
use crate::server::SERVER_RUNTIME;

pub fn get_hostname() -> Result<String> {
    let uname = rustix::system::uname();
    let hostname = uname.nodename();
    Ok(hostname.to_str()?.to_string())
    // let limit = unsafe { libc::sysconf(libc::_SC_HOST_NAME_MAX) };
    // let size = libc::c_long::max(limit, 256) as usize;

    // // Reserve additional space for terminating nul byte.
    // let mut buffer = vec![0u8; size + 1];

    // #[allow(trivial_casts)]
    // let result = unsafe { libc::gethostname(buffer.as_mut_ptr() as *mut libc::c_char, size) };

    // if result != 0 {
    //     return Err(anyhow::anyhow!("gethostname failed"));
    // }

    // let hostname = std::ffi::CStr::from_bytes_until_nul(buffer.as_slice())?;

    // Ok(hostname.to_str()?.to_string())
}

pub fn start_report_worker() {
    SERVER_RUNTIME.spawn(report_worker());
}

async fn report_worker() {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

    loop {
        interval.tick().await;
        if let (Ok(master_addr), Ok(probing_port)) =
            (std::env::var("MASTER_ADDR"), std::env::var("PROBING_PORT"))
        {
            let report_addr = format!("http://{}:{}/apis/nodes", master_addr, probing_port);
            let hostname = get_hostname().unwrap_or("localhost".to_string());
            let local_rank = std::env::var("LOCAL_RANK")
                .unwrap_or("0".to_string())
                .parse()
                .unwrap_or(0);
            let mut address = format!(
                "{}:{}",
                hostname,
                probing_port.parse().unwrap_or(9700) + local_rank
            );
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

            debug!("reporting node status to {report_addr}: {:?}", node);
            match request_remote(&report_addr, node).await {
                Ok(reply) => {
                    debug!("node status reported to {report_addr}: {:?}", reply);
                }
                Err(err) => {
                    eprintln!(
                        "failed to report node status to {master_addr}:{probing_port}, {err}"
                    );
                }
            }
        }
    }
}

fn get_i32_env(name: &str) -> Option<i32> {
    std::env::var(name).unwrap_or_default().parse().ok()
}

async fn request_remote(url: &str, node: Node) -> Result<Vec<u8>> {
    let reply = ureq::put(url)
        .send_json(node)?
        .body_mut()
        .read_to_string()?;
    Ok(reply.into_bytes())
}
