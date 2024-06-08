use std::future::Future;
use std::pin::Pin;

use bytes::Bytes;
use http_body_util::Full;
use hyper::service::Service;
use hyper::{body::Incoming as IncomingBody, Request, Response};

mod asset;
mod process;
mod profiler;
mod python;

#[derive(Default, Clone, Debug)]
pub struct ProbeService {}

impl ProbeService {
    fn route(&self, path: &str, query: Option<&str>) -> Full<Bytes> {
        let path = match path {
            "/" => "/index.html",
            s => s,
        };
        if asset::contains(path) {
            return Full::new(asset::get(path));
        }
        let resp = match path {
            "/apis/overview" => process::overview(),
            "/flamegraph" | "/flamegraph.svg" => profiler::flamegraph(),
            unmatched => python::handle(unmatched, query),
        };
        Full::new(Bytes::from(resp))
    }
}

impl Service<Request<IncomingBody>> for ProbeService {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        let filename = req.uri().path().to_string().clone();
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
        let resp = self.route(req.uri().path(), req.uri().query());
        Box::pin(async { Ok(builder.body(resp).unwrap()) })
    }
}
