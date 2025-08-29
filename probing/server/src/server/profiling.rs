use axum::response::IntoResponse;

use super::error::ApiResult;

/// Generate flamegraph using torch profiler
pub async fn get_torch_flamegraph() -> ApiResult<impl IntoResponse> {
    let graph = probing_python::features::torch::flamegraph();
    Ok((
        [
            ("Content-Type", "image/svg+xml"),
            ("Content-Disposition", "attachment; filename=flamegraph.svg"),
        ],
        graph,
    ))
}

/// Generate flamegraph using pprof
pub async fn get_pprof_flamegraph() -> ApiResult<impl IntoResponse> {
    match probing_python::features::pprof::flamegraph() {
        Ok(graph) => Ok((
            [
                ("Content-Type", "image/svg+xml"),
                ("Content-Disposition", "attachment; filename=flamegraph.svg"),
            ],
            graph,
        )),
        Err(err) => Err(anyhow::anyhow!(err).into()),
    }
}
