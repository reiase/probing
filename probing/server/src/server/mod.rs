mod actors;
mod apis;
mod services;

use std::thread;

use actix_web::{web, App, HttpServer};
use anyhow::Result;
use log::error;
use once_cell::sync::Lazy;

use apis::api_service_config;
use probing_proto::prelude::{ProbeCall, QueryMessage, QueryRequest};
use services::{handle_query, page_service_config, static_files};

pub static SERVER_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    let worker_threads = std::env::var("PROBING_SERVER_WORKER_THREADS")
        .unwrap_or("2".to_string())
        .parse::<usize>()
        .unwrap_or(2);
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

pub async fn local_server() -> Result<()> {
    let prefix_path = std::env::var("PROBING_CTRL_ROOT").unwrap_or("/tmp/probing/".to_string());

    let path = std::path::Path::new(&prefix_path);
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }

    let socket_path = format!("{}/{}", prefix_path, std::process::id());

    let server = match HttpServer::new(|| {
        App::new()
            .service(services::probe)
            .service(services::query)
            .service(web::scope("/apis").configure(api_service_config))
            .configure(page_service_config)
            .route("/{filename:.*}", web::get().to(static_files))
    })
    .workers(2)
    .bind_uds(socket_path.clone())
    {
        Ok(server) => server,
        Err(err) => {
            error!("Failed to bind server to {}: {}", socket_path, err);
            return Err(err.into());
        }
    };
    server.run().await?;
    Ok(())
}

pub fn start_local() {
    SERVER_RUNTIME.spawn(async move {
        let _ = local_server().await;
    });
}

pub async fn remote_server(addr: Option<String>) -> Result<()> {
    let addr = addr.unwrap_or_else(|| "0.0.0.0:0".to_string());
    let server = match HttpServer::new(|| {
        App::new()
            .service(services::probe)
            .service(services::query)
            .service(web::scope("/apis").configure(api_service_config))
            .configure(page_service_config)
            .route("/{filename:.*}", web::get().to(static_files))
    })
    .workers(2)
    .bind(addr.clone())
    {
        Ok(server) => server,
        Err(err) => {
            error!("Failed to bind server to {}: {}", addr.clone(), err);
            return Err(err.into());
        }
    };

    server.run().await?;
    Ok(())
}

pub fn start_remote(addr: Option<String>) {
    SERVER_RUNTIME.spawn(async move {
        let _ = remote_server(addr).await;
    });
}

pub fn sync_env_settings() {
    thread::spawn(|| {
        std::env::vars().for_each(|(k, v)| {
            if k.starts_with("PROBING_") {
                let setting = format!("{}={}", k, v);
                println!("{setting}");
    
                let k = k.replace("_", ".");
                let setting = format!("set {}={}", k, v);
                match handle_query(QueryMessage::Query(QueryRequest {
                    expr: setting.clone(),
                    opts: None,
                })) {
                    Ok(_) => {
                        log::debug!("Synced env setting: {}", setting);
                    }
                    Err(err) => {
                        error!("Failed to sync env settings: {}", err);
                    }
                };
            }
        });
    });
}
