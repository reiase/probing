use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

use anyhow::Result;
use axum::http::StatusCode;
use axum::http::Uri;
use axum::response::AppendHeaders;
use axum::response::IntoResponse;
use axum::response::Response;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use probing_cc::TaskStatsPlugin;
use probing_engine::core::Engine;
use probing_proto::prelude::*;
use probing_python::PythonProbe;

use crate::asset;

pub static PROBE: Lazy<Mutex<Box<dyn Probe>>> =
    Lazy::new(|| Mutex::new(Box::new(PythonProbe::default())));
pub static ENGINE: Lazy<RwLock<Engine>> = Lazy::new(|| {
    use probing_engine::plugins::cluster::ClusterPlugin;
    use probing_python::plugins::python::PythonPlugin;

    let engine = match probing_engine::create_engine()
        // .with_extension_options(ProbingOptions::default())
        .with_plugin("probe", Arc::new(PythonPlugin::new("python")))
        .with_plugin("probe", Arc::new(ClusterPlugin::new("nodes", "cluster")))
        .with_plugin("probe", Arc::new(TaskStatsPlugin::new("taskstats")))
        .with_engine_extension(Arc::new(Mutex::new(
            probing_python::extensions::PprofExtension::default(),
        )))
        .with_engine_extension(Arc::new(Mutex::new(
            probing_python::extensions::TaskStatsExtension::default(),
        )))
        .with_engine_extension(Arc::new(Mutex::new(
            probing_python::extensions::TorchExtension::default(),
        )))
        .with_engine_extension(Arc::new(Mutex::new(
            probing_python::extensions::PythonExtension::default(),
        )))
        .build()
    {
        Ok(engine) => engine,
        Err(e) => {
            log::error!("Error creating engine: {}", e);
            Engine::default()
        }
    };
    RwLock::new(engine)
});

pub fn handle_query(request: QueryMessage) -> Result<QueryMessage> {
    // use probing_engine::plugins::cluster::ClusterPlugin;
    // use probing_python::plugins::python::PythonPlugin;

    if let QueryMessage::Query { expr, opts: _ } = request {
        let resp = thread::spawn(move || {
            // let engine = probing_engine::create_engine()
            //     .with_extension_options(ProbingOptions::default())
            //     .with_plugin("probe", Arc::new(PythonPlugin::new("python")))
            //     .with_plugin("probe", Arc::new(ClusterPlugin::new("nodes", "cluster")))
            //     .with_plugin("probe", Arc::new(TaskStatsPlugin::new("taskstats")))
            //     .build()?;

            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let q = expr.clone();
                    let engine = ENGINE.read().await;

                    if q.starts_with("set") && q.contains(";") {
                        for q in q.split(";") {
                            match engine.sql(q).await {
                                Ok(_) => {}
                                Err(e) => {
                                    log::error!("Error executing query: {}", e);
                                }
                            };
                        }
                        Ok(vec![])
                    } else {
                        engine.execute(&expr, "ron")
                    }
                })
        })
        .join()
        .map_err(|_| anyhow::anyhow!("error joining thread"))??;

        Ok(QueryMessage::Reply {
            data: resp,
            format: QueryDataFormat::RON,
        })
    } else {
        Err(anyhow::anyhow!("Invalid query message"))
    }
}

// #[post("/probe")]
pub async fn probe(
    axum::extract::RawForm(req): axum::extract::RawForm,
) -> Result<impl IntoResponse, AppError> {
    let probe = PROBE.lock().unwrap();
    let request = ron::from_str::<ProbeCall>(String::from_utf8(req.to_vec())?.as_str());
    let request = match request {
        Ok(request) => request,
        Err(err) => return Err(anyhow::anyhow!(err.to_string()).into()),
    };
    let reply = probe.ask(request);
    let reply = match ron::to_string(&reply) {
        Ok(reply) => reply,
        Err(err) => return Err(anyhow::anyhow!(err.to_string()).into()),
    };
    Ok(reply)
}

pub async fn query(req: String) -> Result<String, AppError> {
    let request = ron::from_str::<QueryMessage>(&req);
    let request = match request {
        Ok(request) => request,
        Err(err) => return Err(anyhow::anyhow!(err.to_string()).into()),
    };

    let reply = match handle_query(request) {
        Ok(reply) => reply,
        Err(err) => QueryMessage::Error {
            message: err.to_string(),
        },
    };

    Ok(ron::to_string(&reply)?)
}

pub async fn index() -> impl IntoResponse {
    (
        AppendHeaders([("Content-Type", "text/html")]),
        asset::get("/index.html"),
    )
}

pub async fn static_files(filename: Uri) -> Result<impl IntoResponse, StatusCode> {
    let filename = filename.path();
    if !asset::contains(filename) {
        return Err(StatusCode::NOT_FOUND);
    }
    log::debug!("serving file: {}", filename);
    Ok((
        AppendHeaders([(
            "Content-Type",
            match &filename {
                p if p.ends_with(".html") => "text/html",
                p if p.ends_with(".js") => "application/javascript",
                p if p.ends_with(".css") => "text/css",
                p if p.ends_with(".svg") => "image/svg+xml",
                p if p.ends_with(".wasm") => "application/wasm",
                _ => "text/html",
            },
        )]),
        asset::get(filename),
    ))
}

// Make our own error that wraps `anyhow::Error`.
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
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
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
