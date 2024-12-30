use std::sync::Arc;

use anyhow::Result;
use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, Method, Request, Response};

use log::debug;

use probing_core::Probe;
use probing_legacy::service::handle_request as legacy_handle_request;

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

        (&Method::GET, "/probe") => {
            let request = request.collect().await?.to_bytes().to_vec();
            let response = probe.handle(request.as_slice());

            let body = match response {
                Ok(response) => Full::new(Bytes::from(response)),
                Err(e) => Full::new(Bytes::from(format!("Error: {}", e))),
            };
            Ok(Response::builder().body(body)?)
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
        _ => legacy_handle_request(request).await,
    }
}
