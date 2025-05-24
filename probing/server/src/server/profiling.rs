use axum::{
    response::{AppendHeaders, IntoResponse},
};
use probing_python::{flamegraph::flamegraph, pprof::PPROF_HOLDER};

use super::error::ApiResult;

/// Generate flamegraph using torch profiler
pub async fn get_torch_flamegraph() -> ApiResult<impl IntoResponse> {
    let graph = flamegraph();
    Ok((
        AppendHeaders([
            ("Content-Type", "image/svg+xml"),
            ("Content-Disposition", "attachment; filename=flamegraph.svg"),
        ]),
        graph,
    ))
}

/// Generate flamegraph using pprof
pub async fn get_pprof_flamegraph() -> ApiResult<impl IntoResponse> {
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
