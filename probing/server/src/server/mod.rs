mod actors;
mod apis;
mod services;

use std::thread;

use actix_web::{web, App, HttpServer};
use log::error;
use log::info;
use once_cell::sync::Lazy;

use apis::api_service_config;
use services::{page_service_config, static_files};

pub static SERVER_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    let worker_threads = std::env::var("PROBING_SERVER_WORKER_THREADS")
        .unwrap_or("2".to_string())
        .parse::<usize>()
        .unwrap_or(2);
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(worker_threads)
        .build()
        .unwrap()
});

pub async fn local_server() -> std::io::Result<()> {
    let prefix_path = std::env::var("PROBING_CTRL_ROOT").unwrap_or("/tmp/probing/".to_string());

    let path = std::path::Path::new(&prefix_path);
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }

    let socket_path = format!("{}/{}", prefix_path, std::process::id());

    HttpServer::new(|| {
        App::new()
            .service(services::probe)
            .service(services::query)
            .service(web::scope("/apis").configure(api_service_config))
            .configure(page_service_config)
            .route("/{filename:.*}", web::get().to(static_files))
    })
    .bind_uds(socket_path)?
    .workers(2)
    .run()
    .await
}

pub fn start_local() {
    thread::spawn(move || {
        SERVER_RUNTIME.block_on(async move {
            match local_server().await {
                Ok(_) => info!("Local server started successfully."),
                Err(err) => error!("Failed to start local server: {}", err),
            }
        });
    });
}

pub async fn remote_server(addr: Option<String>) -> std::io::Result<()> {
    let addr = addr.unwrap_or_else(|| "0.0.0.0:0".to_string());
    HttpServer::new(|| {
        App::new()
            .service(services::probe)
            .service(services::query)
            .service(web::scope("/apis").configure(api_service_config))
            .configure(page_service_config)
            .route("/{filename:.*}", web::get().to(static_files))
    })
    .bind(addr)?
    .workers(2)
    .run()
    .await
}

pub fn start_remote(addr: Option<String>) {
    thread::spawn(move || {
        SERVER_RUNTIME.block_on(async move {
            match remote_server(addr).await {
                Ok(_) => info!("Remote server started successfully."),
                Err(err) => error!("Failed to start remote server: {}", err),
            }
        });
    });
}
