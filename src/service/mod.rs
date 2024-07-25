use std::collections::HashMap;

use anyhow::Result;
use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, Method, Request, Response};

mod asset;
mod process;
mod profiler;
mod python;

use log::debug;
use probing_common::cli::CtrlSignal;
pub use process::CALLSTACK;

use crate::ctrl::{ctrl_handler_string, handle_ctrl};

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
        (&Method::POST, "/ctrl") => {
            let whole_body = String::from_utf8(req.collect().await?.to_bytes().to_vec());
            if let Ok(cmdstr) = whole_body {
                if cmdstr.starts_with('[') {
                    ctrl_handler_string(cmdstr);
                    Ok(Default::default())
                } else if let Ok(ctrl) = ron::from_str::<CtrlSignal>(&cmdstr) {
                    match handle_ctrl(ctrl) {
                        Ok(resp) => {
                            let resp = Full::new(Bytes::from(resp));
                            Ok(Response::builder().body(resp).unwrap())
                        }
                        Err(_) => anyhow::bail!("internal error!"),
                    }
                } else {
                    anyhow::bail!("internal error!")
                }
            } else {
                Ok(Default::default())
            }
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
