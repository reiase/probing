use anyhow::{self, Result};
use log;
use probing_proto::prelude::*;

use crate::extensions as se;
use probing_cc::extensions as cc;
use probing_python::extensions as py;

use crate::server::error::ApiResult;

pub use probing_core::ENGINE;

pub async fn initialize_engine() -> Result<()> {
    let builder = probing_core::create_engine()
        .with_extension(py::PprofExtension::default(), "pprof", None)
        .with_extension(py::TorchExtension::default(), "torch", None)
        .with_extension(se::ServerExtension::default(), "server", None)
        .with_extension(py::PythonExt::default(), "python", None)
        .with_extension(cc::TaskStatsExtension::default(), "taskstats", None)
        .with_extension(cc::ClusterExtension::default(), "cluster", Some("nodes"))
        .with_extension(cc::EnvExtension::default(), "process", Some("envs"))
        .with_extension(cc::FilesExtension::default(), "files", None);

    probing_core::initialize_engine(builder).await
}

pub async fn handle_query(request: Query) -> Result<QueryDataFormat> {
    let Query { expr, opts: _ } = request;

    // No more thread::spawn or block_on needed here.
    // We are already running within the Axum/Tokio runtime.

    // Acquire the engine lock asynchronously
    let engine = ENGINE.read().await;

    if expr.starts_with("set ") || expr.starts_with("SET ") {
        // Split potentially multiple SET statements
        for q in expr.split(';').filter(|s| !s.trim().is_empty()) {
            let trimmed_q = q.trim();
            if trimmed_q.is_empty() {
                continue;
            }
            log::debug!("Executing SET statement: {}", trimmed_q);
            // Execute the SQL statement asynchronously
            match engine.sql(trimmed_q).await {
                Ok(_) => {
                    log::debug!("Successfully executed: {}", trimmed_q);
                }
                Err(e) => {
                    // Log the error and potentially return it
                    log::error!("Error executing SET statement '{}': {}", trimmed_q, e);
                    // Depending on requirements, you might want to stop processing
                    // or collect errors. For now, just log and continue.
                    // Or return an error immediately:
                    // return Err(anyhow::anyhow!("Failed SET query '{}': {}", trimmed_q, e));
                }
            };
        }
        // Return Nil even if some SET statements failed (adjust if needed)
        Ok(QueryDataFormat::Nil)
    } else {
        log::debug!("Executing SELECT query: {}", expr);
        // Use the fully async query method and await it
        match engine.async_query(&expr).await {
            Ok(dataframe) => Ok(QueryDataFormat::DataFrame(dataframe)),
            Err(e) => {
                log::error!("Error executing SELECT query '{}': {}", expr, e);
                // Convert DataFusionError/EngineError into anyhow::Error
                Err(e.into())
            }
        }
    }
}

// 处理Web API查询请求
pub async fn query(req: String) -> ApiResult<String> {
    let request = serde_json::from_str::<Message<Query>>(&req);
    let request = match request {
        Ok(request) => request.payload,
        Err(err) => {
            log::error!("Failed to deserialize query request: {}", err);
            return Err(anyhow::anyhow!("Invalid request format: {}", err).into());
        }
    };

    // Await the async handle_query function
    let reply_payload = match handle_query(request).await {
        Ok(reply) => reply,
        Err(err) => {
            // Error already logged in handle_query if it originated there
            QueryDataFormat::Error(QueryError {
                code: ErrorCode::Internal,
                message: err.to_string(),
                details: None,
            })
        }
    };

    // Wrap the payload in a Message
    let reply_message = Message::new(reply_payload);

    // Serialize the response message
    serde_json::to_string(&reply_message).map_err(|e| {
        log::error!("Failed to serialize query response: {}", e);
        anyhow::anyhow!("Failed to create response: {}", e).into() // Convert to ApiError
    })
}
