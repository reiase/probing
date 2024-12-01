use std::thread;

use probing_dpp::protocol::cluster::Node;

use crate::get_hostname;

pub fn start_report_worker() {
    thread::spawn(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(report_worker());
    });
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
            let local_rank = std::env::var("LOCAL_RANK").unwrap_or("0".to_string()).parse().unwrap_or(0);
            let address = format!("{}:{}", hostname, probing_port.parse().unwrap_or(9700)+local_rank);
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
            };

            if let Err(err) = reqwest::Client::new()
                .put(&report_addr)
                .body(ron::to_string(&node).unwrap())
                .send()
                .await
            {
                eprintln!(
                    "failed to report node status to {}: {err}",
                    format!("{}:{}", master_addr, probing_port),
                );
            }
        }
    }
}

fn get_i32_env(name: &str) -> Option<i32> {
    std::env::var(name).unwrap_or_default().parse().ok()
}