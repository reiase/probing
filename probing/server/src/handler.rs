use std::sync::Arc;

use anyhow::Result;
use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, Method, Request, Response};

use log::debug;

use probing_proto::protocol::probe::Probe;
use probing_proto::prelude::QueryDataFormat;
use probing_proto::prelude::QueryMessage;
use probing_proto::prelude::QueryReply;

use crate::asset;

pub async fn handle_request(
    request: Request<hyper::body::Incoming>,
    probe: Arc<dyn Probe>,
) -> Result<Response<Full<Bytes>>> {
    debug!(
        "requesting: {:?} {}",
        request.method(),
        request.uri().path()
    );
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/")
        | (&Method::GET, "/cluster")
        | (&Method::GET, "/overview")
        | (&Method::GET, "/activity")
        | (&Method::GET, "/inspect")
        | (&Method::GET, "/index.html") => Ok(Response::builder()
            .header("Content-Type", "text/html")
            .body(Full::new(asset::get("/index.html")))
            .unwrap()),

        (&Method::POST, "/probe") => {
            let request = request.collect().await?.to_bytes().to_vec();
            let response = probe.handle(request.as_slice());

            let body = match response {
                Ok(response) => Full::new(Bytes::from(response)),
                Err(e) => Full::new(Bytes::from(format!("Error: {}", e))),
            };
            Ok(Response::builder().body(body)?)
        }

        (&Method::POST, "/query") => {
            let request = request.collect().await?.to_bytes().to_vec();
            let request = String::from_utf8(request)?;
            let request = ron::from_str::<QueryMessage>(&request)?;

            match handle_query(request) {
                Ok(resp) => {
                    let resp = Full::new(Bytes::from(resp));
                    Ok(Response::builder().body(resp).unwrap())
                }
                Err(err) => {
                    let resp = err.to_string();
                    let resp = Full::new(Bytes::from(resp));
                    Ok(Response::builder().status(500).body(resp).unwrap())
                }
            }
        }

        (&Method::GET, filename) if asset::contains(filename) => {
            let body = Full::new(asset::get(filename));
            let mime = match filename {
                p if p.ends_with(".html") => Some("text/html"),
                p if p.ends_with(".js") => Some("application/javascript"),
                p if p.ends_with(".css") => Some("text/css"),
                p if p.ends_with(".svg") => Some("image/svg+xml"),
                p if p.ends_with(".wasm") => Some("application/wasm"),
                _ => None,
            };
            let builder = if let Some(mime) = mime {
                Response::builder().header("Content-Type", mime)
            } else {
                Response::builder()
            };
            Ok(builder.body(body)?)
        }
        _ => todo!("unsupported request: {:?}", request),
        // _ => legacy_handle_request(request).await,
    }
}

pub fn handle_query(query: QueryMessage) -> Result<Vec<u8>> {
    use probing_engine::plugins::cluster::ClusterPlugin;
    use probing_python::plugins::python::PythonPlugin;

    let engine = probing_engine::create_engine();
    engine.enable("probe", Arc::new(PythonPlugin::new("python")))?;
    engine.enable("probe", Arc::new(ClusterPlugin::new("nodes", "cluster")))?;
    if let QueryMessage::Query(query) = query {
        let resp = engine.execute(&query.expr, "ron")?;
        Ok(ron::to_string(&QueryMessage::Reply(QueryReply {
            data: resp,
            format: QueryDataFormat::RON,
        }))?
        .as_bytes()
        .to_vec())
    } else {
        Err(anyhow::anyhow!("Invalid query message"))
    }
}
