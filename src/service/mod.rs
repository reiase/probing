use std::collections::HashMap;
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

pub use process::CALLSTACK;

#[derive(Default, Clone, Debug)]
pub struct ProbeService {}

impl ProbeService {
    fn parse_qs(&self, qs: Option<&str>) -> HashMap<String, String> {
        if let Some(qs) = qs {
            let qs = if qs.starts_with('?') {
                qs.to_string()
            } else {
                format!("?{}", qs)
            };
            let qs: HashMap<String, String> =
                qstring::QString::from(qs.as_str()).into_iter().collect();
            qs
        } else {
            Default::default()
        }
    }

    fn route(&self, path: &str, query: Option<&str>) -> Full<Bytes> {
        let params = self.parse_qs(query);
        let path = match path {
            "/" => "/index.html",
            s => s,
        };
        if asset::contains(path) {
            return Full::new(asset::get(path));
        }
        let resp = match path {
            "/apis/overview" => process::overview(),
            "/apis/callstack" => process::callstack(params.get("tid").cloned()),
            "/apis/files" => process::files(params.get("path").cloned()),
            "/apis/flamegraph" | "/flamegraph.svg" => profiler::flamegraph(),
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
