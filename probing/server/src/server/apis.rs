use std::collections::HashMap;

use anyhow::Result;

use axum::{
    http::StatusCode,
    response::{AppendHeaders, IntoResponse, Response},
    Router,
};
use probing_proto::prelude::*;
use probing_python::pprof::PPROF_HOLDER;

use crate::server::services::PROBE;

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
                let envs: Vec<String> = m
                    .iter()
                    .map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy()))
                    .collect();
                envs.join("\n")
            })
            .unwrap_or("".to_string()),
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
    let probe = PROBE.clone();
    let retval = probe.send(ProbeCall::CallFlamegraph).await;

    match retval {
        Ok(ProbeCall::ReturnFlamegraph(flamegraph)) => Ok((
            AppendHeaders([
                ("Content-Type", "image/svg+xml"),
                ("Content-Disposition", "attachment; filename=flamegraph.svg"),
            ]),
            flamegraph,
        )),
        Ok(ProbeCall::Err(err)) => Err(anyhow::anyhow!(err).into()),
        _ => Err(anyhow::anyhow!("unexpected response from probe: {:?}", retval).into()),
    }
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

async fn api_get_callstack(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<String, ApiError> {
    let tid: Option<i32> = params.get("tid").map(|x| x.parse().unwrap_or_default());
    let probe = crate::server::services::PROBE.clone();

    let reply = match probe.send(ProbeCall::CallBacktrace(tid)).await {
        Ok(reply) => reply,
        Err(err) => ProbeCall::Err(err.to_string()),
    };
    Ok(serde_json::to_string(&reply)?)
}

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
    use probing_engine::plugins::cluster::service::update_node;
    update_node(payload);
    Ok(())
}

async fn get_nodes() -> Result<String, ApiError> {
    use probing_engine::plugins::cluster::service::get_nodes;
    let nodes = get_nodes();
    Ok(serde_json::to_string(&nodes)?)
}

pub fn apis_route() -> axum::Router {
    Router::new()
        .route("/overview", axum::routing::get(api_get_overview))
        .route("/callback", axum::routing::get(api_get_callstack))
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
