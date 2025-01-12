mod actors;
mod services;

use std::thread;

use actix_web::{web, App, HttpServer};

use log::error;
use log::info;

use services::{api_service_config, page_service_config, static_files};

pub async fn local_server() -> std::io::Result<()> {
    let socket_path = std::env::var("PROBING_CTRL_ROOT").unwrap_or("/tmp/probing/".to_string());
    let socket_path = format!("{}/{}", socket_path, std::process::id());

    HttpServer::new(|| {
        App::new()
            .service(services::probe)
            .service(services::query)
            .service(web::scope("/api").configure(api_service_config))
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
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
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
            .service(web::scope("/api").configure(api_service_config))
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
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                match remote_server(addr).await {
                    Ok(_) => info!("Remote server started successfully."),
                    Err(err) => error!("Failed to start remote server: {}", err),
                }
            });
    });
}
