use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use anyhow::Result;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::service::Service;
use hyper::Method;
use hyper::{body::Incoming as IncomingBody, Request, Response};

mod asset;
mod process;
mod profiler;
mod python;

use probing_common::cli::ProbingCommand;
pub use process::CALLSTACK;

use crate::ctrl::ctrl_handler_string;

#[derive(Default, Clone, Debug)]
pub struct ProbingService {}

fn parse_qs(qs: Option<&str>) -> HashMap<String, String> {
    if let Some(qs) = qs {
        let qs = if qs.starts_with('?') {
            qs.to_string()
        } else {
            format!("?{}", qs)
        };
        let qs: HashMap<String, String> = qstring::QString::from(qs.as_str()).into_iter().collect();
        qs
    } else {
        Default::default()
    }
}

impl ProbingService {
    fn parse_qs(&self, qs: Option<String>) -> HashMap<String, String> {
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

    fn route(&self, path: &str, query: Option<String>, body: String) -> Full<Bytes> {
        let params = self.parse_qs(query.clone());
        let path = match path {
            "/" => "/index.html",
            s => s,
        };
        if asset::contains(path) {
            return Full::new(asset::get(path));
        }
        let resp = match path {
            "/ctrl" => {
                crate::ctrl::ctrl_handler_string(body);
                Default::default()
            }
            "/apis/overview" => process::overview(),
            "/apis/callstack" => process::callstack(params.get("tid").cloned()),
            "/apis/files" => process::files(params.get("path").cloned()),
            "/apis/flamegraph" | "/flamegraph.svg" => profiler::flamegraph(),
            unmatched => python::handle(unmatched, query),
        };
        Full::new(Bytes::from(resp))
    }
}

impl Service<Request<IncomingBody>> for ProbingService {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        let path = req.uri().path().to_string();
        let qstr = req.uri().query().map(|qstr| qstr.to_string());
        let filename = path.to_string().clone();
        let mime = match filename {
            p if p.ends_with(".html") => Some("text/html"),
            p if p.ends_with(".js") => Some("application/javascript"),
            p if p.ends_with(".css") => Some("text/css"),
            p if p.ends_with(".svg") => Some("image/svg+xml"),
            p if p.ends_with(".wasm") => Some("application/wasm"),
            _ => None,
        };

        let body = "".to_string();

        let builder = if let Some(mime) = mime {
            Response::builder().header("Content-Type", mime)
        } else {
            Response::builder()
        };
        let resp = self.route(path.as_str(), qstr, body);
        Box::pin(async { Ok(builder.body(resp).unwrap()) })
    }
}

pub async fn handle_request(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>> {
    let params = parse_qs(req.uri().query());
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/ctrl") => {
            let whole_body = String::from_utf8(req.collect().await?.to_bytes().to_vec());
            if let Ok(cmdstr) = whole_body {
                ctrl_handler_string(cmdstr);
            }
            Ok(Default::default())
        }

        (&Method::GET, "/") | (&Method::GET, "/index.html") => Ok(Response::builder()
            .header("Content-Type", "text/html")
            .body(Full::new(asset::get("/index.html")))
            .unwrap()),

        (&Method::GET, "/apis/overview") => {
            let resp = process::overview();
            let resp = Full::new(Bytes::from(resp));
            Ok(Response::builder().body(resp).unwrap())
        }

        (&Method::GET, "/apis/callstack") => {
            let resp = process::callstack(params.get("tid").cloned());
            let resp = Full::new(Bytes::from(resp));
            Ok(Response::builder().body(resp).unwrap())
        }

        (&Method::GET, "/apis/files") => {
            let resp = process::files(params.get("path").cloned());
            let resp = Full::new(Bytes::from(resp));
            Ok(Response::builder().body(resp).unwrap())
        }

        (&Method::GET, "/apis/flamegraph") => {
            let resp = profiler::flamegraph();
            let resp = Full::new(Bytes::from(resp));
            Ok(Response::builder().body(resp).unwrap())
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
            Ok(builder.body(body).unwrap())
        }
        (&Method::GET, path) => {
            let resp = python::handle(path, req.uri().query().map(|x| x.to_string()));
            let resp = Full::new(Bytes::from(resp));
            Ok(Response::builder().body(resp).unwrap())
        }
        _ => Ok(Default::default()),
    }
}
