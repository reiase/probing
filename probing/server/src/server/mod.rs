mod apis;
pub mod cluster;
pub mod config;
pub mod error;
pub mod extension_handler;
pub mod file_api;
pub mod middleware;
pub mod profiling;
pub mod system;

use anyhow::Result;
use apis::apis_route;
use log::error;
use once_cell::sync::Lazy;

use crate::asset::{index, static_files};
use crate::engine::{handle_query, initialize_engine};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use middleware::{request_logging_middleware, request_size_limit_middleware};
use probing_proto::prelude::Query;

async fn get_config_value_handler(
    axum::extract::Path(config_key): axum::extract::Path<String>,
) -> impl IntoResponse {
    match probing_core::config::get(&config_key).await {
        Ok(value) => (StatusCode::OK, value).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error retrieving config '{}': {}", config_key, e),
        )
            .into_response(),
    }
}

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

fn build_app(auth: bool) -> axum::Router {
    let mut app = axum::Router::new()
        .route("/", axum::routing::get(index))
        .route("/overview", axum::routing::get(index))
        .route("/cluster", axum::routing::get(index))
        .route("/activity", axum::routing::get(index))
        .route("/inspect", axum::routing::get(index))
        .route("/timeseries", axum::routing::get(index))
        .route("/index.html", axum::routing::get(index))
        .route("/profiler", axum::routing::get(index))
        .route("/query", axum::routing::post(query))
        .route(
            "/config/{config_key}",
            axum::routing::get(get_config_value_handler),
        )
        .nest_service("/apis", apis_route())
        .fallback(static_files)
        // Apply request size limiting middleware
        .layer(axum::middleware::from_fn(request_size_limit_middleware))
        // Apply request logging middleware (optional, for debugging)
        .layer(axum::middleware::from_fn(request_logging_middleware));

    // Apply authentication middleware if auth token is configured
    if auth {
        app = app.layer(axum::middleware::from_fn(
            crate::auth::selective_auth_middleware,
        ));
    }

    app
}

/// HTTP handler wrapper for query endpoint
async fn query(body: String) -> impl IntoResponse {
    match crate::engine::query(body).await {
        Ok(response) => (StatusCode::OK, response).into_response(),
        Err(api_error) => api_error.into_response(),
    }
}

pub async fn local_server() -> Result<()> {
    let socket_path = format!("\0probing-{}", std::process::id());

    eprintln!("Starting local server at {}", socket_path);

    let app = build_app(false);
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

    let app = build_app(true);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    match listener.local_addr() {
        Ok(addr) => {
            {
                let mut probing_address = crate::vars::PROBING_ADDRESS.write().unwrap();
                *probing_address = addr.to_string();
            }
            eprintln!("{}", Red.bold().paint("probing server is available on:"));
            eprintln!("\t{}", Green.bold().underline().paint(addr.to_string()));
            probing_core::config::set("server.address", &addr.to_string()).await?;
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
                    "PROBING_AUTH_TOKEN", // Skip syncing the auth token for security reasons
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
                expr: setting,
                opts: None,
            })
            .await
            {
                Ok(_) => {
                    log::debug!("Synced env setting: {}", k);
                }
                Err(err) => {
                    error!("Failed to sync env settings: set {}={}, {err}", k, v);
                }
            };
        }
    });
}
