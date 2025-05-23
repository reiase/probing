use std::collections::HashMap;

use anyhow::Result;

use axum::{
    http::StatusCode,
    response::{AppendHeaders, IntoResponse, Response},
    Router,
};
use http_body_util::BodyExt;

use probing_core::core::EngineExtensionManager;
use probing_proto::prelude::*;
use probing_python::{flamegraph::flamegraph, pprof::PPROF_HOLDER};

use crate::server::services::ENGINE;

pub fn overview() -> Result<Process> {
    let current = procfs::process::Process::myself()?;
    let info = Process {
        pid: current.pid(),
        exe: current
            .exe()
            .map(|exe| exe.to_string_lossy().to_string())
            .unwrap_or("nil".to_string()),
        env: current
            .environ()
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.to_string_lossy().to_string(), v.to_string_lossy().to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        cmd: current
            .cmdline()
            .map(|cmds| cmds.join(" "))
            .unwrap_or("".to_string()),
        cwd: current
            .cwd()
            .map(|cwd| cwd.to_string_lossy().to_string())
            .unwrap_or("".to_string()),
        main_thread: current
            .task_main_thread()
            .map(|p| p.pid as u64)
            .unwrap_or(0),
        threads: current
            .tasks()
            .map(|iter| iter.map(|r| r.map(|p| p.tid as u64).unwrap_or(0)).collect())
            .unwrap_or_default(),
    };
    Ok(info)
}

async fn api_get_overview() -> Result<String, ApiError> {
    Ok(serde_json::to_string(&overview()?)?)
}

async fn api_get_flamegraph_torch() -> Result<impl IntoResponse, ApiError> {
    let graph = flamegraph();
    Ok((
        AppendHeaders([
            ("Content-Type", "image/svg+xml"),
            ("Content-Disposition", "attachment; filename=flamegraph.svg"),
        ]),
        graph,
    ))
}

async fn api_get_flamegraph_pprof() -> Result<impl IntoResponse, ApiError> {
    match PPROF_HOLDER.flamegraph() {
        Ok(graph) => Ok((
            AppendHeaders([
                ("Content-Type", "image/svg+xml"),
                ("Content-Disposition", "attachment; filename=flamegraph.svg"),
            ]),
            graph,
        )),
        Err(err) => Err(anyhow::anyhow!(err).into()),
    }
}

// async fn api_get_callstack(
//     axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
// ) -> Result<String, ApiError> {
//     let tid: Option<i32> = params.get("tid").map(|x| x.parse().unwrap_or_default());
//     let probe = PROBE.lock().map_err(|e| {
//         log::error!("error locking probe: {}", e);
//         anyhow::anyhow!("error locking probe: {}", e)
//     })?;

//     let reply = probe.ask(ProbeCall::CallBacktrace(tid));
//     Ok(serde_json::to_string(&reply)?)
// }

async fn api_get_files(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<String, ApiError> {
    let path = params.get("path");
    if let Some(path) = path {
        let content = std::fs::read_to_string(path)?;
        Ok(content)
    } else {
        Ok("".to_string())
    }
}

async fn put_nodes(axum::Json(payload): axum::Json<Node>) -> Result<(), ApiError> {
    use probing_core::core::cluster::update_node;
    update_node(payload);
    Ok(())
}

async fn get_nodes() -> Result<String, ApiError> {
    use probing_core::core::cluster::get_nodes;
    let nodes = get_nodes();
    Ok(serde_json::to_string(&nodes)?)
}

async fn extension_call(req: axum::extract::Request) -> Result<axum::response::Response, ApiError> {
    let (parts, body) = req.into_parts();
    let path = parts.uri.path();
    let params_str = parts.uri.query().unwrap_or_default();
    let params: HashMap<String, String> =
        serde_urlencoded::from_str(params_str).unwrap_or_default();
    let body = body.collect().await?.to_bytes().clone();

    log::info!(
        "API Call[{}]: params = {:?}, body = {:?}",
        path,
        params,
        body
    );

    let engine = ENGINE.write().await;
    let state = engine.context.state();
    let eem = state
        .config()
        .options()
        .extensions
        .get::<EngineExtensionManager>();
    if let Some(eem) = eem {
        match eem.call(path, &params, body.to_vec().as_slice()) {
            Ok(response) => {
                // If response is a string, return it as plain text
                return Ok((
                    StatusCode::OK,
                    AppendHeaders([("Content-Type", "text/plain")]),
                    response,
                )
                    .into_response());
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Extension call failed: {}", e).into());
            }
        }
    }

    // Return 404 if no extension manager is available
    Ok((StatusCode::NOT_FOUND, "Extension not found").into_response())
}

pub fn apis_route() -> axum::Router {
    Router::new()
        .route("/overview", axum::routing::get(api_get_overview))
        .route("/files", axum::routing::get(api_get_files))
        .route("/nodes", axum::routing::get(get_nodes).put(put_nodes))
        .route(
            "/flamegraph/torch",
            axum::routing::get(api_get_flamegraph_torch),
        )
        .route(
            "/flamegraph/pprof",
            axum::routing::get(api_get_flamegraph_pprof),
        )
        .fallback(extension_call)
}

// Make our own error that wraps `anyhow::Error`.
struct ApiError(anyhow::Error);

// Tell axum how to convert `ApiError` into a response.
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
