mod apis;
mod services;

use anyhow::Result;
use apis::apis_route;
use log::error;
use once_cell::sync::Lazy;

use probing_proto::prelude::Query;
use services::{handle_query, index, initialize_engine, query, static_files};

pub static SERVER_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    let worker_threads = std::env::var("PROBING_SERVER_WORKER_THREADS")
        .unwrap_or("4".to_string())
        .parse::<usize>()
        .unwrap_or(4);
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(worker_threads)
        .thread_name("server runtime")
        .on_thread_start(|| {
            log::debug!(
                "start server runtime thread: {:?}",
                std::thread::current().id()
            );
        })
        .build()
        .unwrap()
});

fn build_app() -> axum::Router {
    axum::Router::new()
        .route("/", axum::routing::get(index))
        .route("/overview", axum::routing::get(index))
        .route("/cluster", axum::routing::get(index))
        .route("/activity", axum::routing::get(index))
        .route("/inspect", axum::routing::get(index))
        .route("/timeseries", axum::routing::get(index))
        .route("/index.html", axum::routing::get(index))
        .route("/profiler", axum::routing::get(index))
        .route("/query", axum::routing::post(query))
        .nest_service("/apis", apis_route())
        .fallback(static_files)
}

pub async fn local_server() -> Result<()> {
    let socket_path = format!("\0probing-{}", std::process::id());

    eprintln!("Starting local server at {}", socket_path);
    let app = build_app();
    axum::serve(tokio::net::UnixListener::bind(socket_path)?, app).await?;
    Ok(())
}

pub fn start_local() {
    SERVER_RUNTIME.block_on(async move {
        initialize_engine()
            .await
            .unwrap_or_else(|err| error!("Failed to initialize engine: {err}"));
    });
    SERVER_RUNTIME.spawn(async move {
        let _ = local_server().await;
    });
}

pub async fn remote_server(addr: Option<String>) -> Result<()> {
    use nu_ansi_term::Color::{Green, Red};

    let addr = addr.unwrap_or_else(|| "0.0.0.0:0".to_string());
    log::info!("Starting probe server at {}", addr);

    let app = build_app();
    let listener = tokio::net::TcpListener::bind(addr).await?;

    match listener.local_addr() {
        Ok(addr) => {
            {
                let mut probing_address = crate::vars::PROBING_ADDRESS.write().unwrap();
                *probing_address = addr.to_string();
            }
            use std::io::Write;
            let mut stderr = std::io::stderr().lock(); // 获取锁

            let _ = writeln!(stderr, "{}", Red.bold().paint("probing server is available on:"));
            let _ = writeln!(stderr,"\t{}", Green.bold().underline().paint(addr.to_string()));
        }
        Err(err) => {
            eprintln!(
                "{}",
                Red.bold()
                    .paint(format!("error getting server address: {err}"))
            );
        }
    }
    axum::serve(listener, app).await?;

    Ok(())
}

pub fn start_remote(addr: Option<String>) {
    SERVER_RUNTIME.spawn(async move {
        let _ = remote_server(addr).await;
    });
}

pub fn sync_env_settings() {
    // Collect environment variables before spawning the async task
    let env_vars: Vec<(String, String)> = std::env::vars()
        .filter(|(k, _)| {
            k.starts_with("PROBING_")
                && ![
                    "PROBING_PORT",
                    "PROBING_LOGLEVEL",
                    "PROBING_ASSETS_ROOT",
                    "PROBING_SERVER_ADDRPATTERN",
                ]
                .contains(&k.as_str())
        })
        .collect();

    // Spawn the task onto the existing Tokio runtime
    SERVER_RUNTIME.spawn(async move {
        for (k, v) in env_vars {
            let k = k.replace("_", ".").to_lowercase();
            let setting = format!("set {}={}", k, v);
            // Since handle_query might not be async itself, but interacts with
            // components managed by the runtime, it's safer to run it within
            // the runtime's context. If handle_query becomes async, add .await
            match handle_query(Query {
                expr: setting.clone(),
                opts: None,
            })
            .await
            {
                Ok(_) => {
                    log::debug!("Synced env setting: {}", setting);
                }
                Err(err) => {
                    error!("Failed to sync env settings: {setting}, {err}");
                }
            };
        }
    });
}
