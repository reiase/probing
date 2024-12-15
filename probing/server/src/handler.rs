use std::collections::HashMap;

use anyhow::Result;
use http_body_util::Full;
use hyper::{body::Bytes, Method, Request, Response};

use log::debug;

use crate::asset;

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

pub async fn handle_request(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>> {
    let params = parse_qs(req.uri().query());
    debug!("requesting: {:?} {}", req.method(), req.uri().path());
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/")
        | (&Method::GET, "/cluster")
        | (&Method::GET, "/overview")
        | (&Method::GET, "/activity")
        | (&Method::GET, "/inspect")
        | (&Method::GET, "/index.html") => Ok(Response::builder()
            .header("Content-Type", "text/html")
            .body(Full::new(asset::get("/index.html")))
            .unwrap()),

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
        _ => Ok(Default::default()),
    }
}