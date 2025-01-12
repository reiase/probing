use actix::Actor;
use actix::Addr;
use actix_web::http::header;
use actix_web::HttpRequest;
use actix_web::{post, web, HttpResponse, Responder};
use once_cell::sync::Lazy;
use probing_proto::prelude::*;
use probing_python::PythonProbe;

use crate::asset;
use crate::handler::handle_query;

use super::actors::ProbeActor;

pub static PROBE: Lazy<Addr<ProbeActor>> =
    Lazy::new(|| ProbeActor::new(Box::new(PythonProbe::default())).start());

#[post("/probe")]
async fn probe(req: String) -> impl Responder {
    let probe = PROBE.clone();
    let request = ron::from_str::<ProbeCall>(&req);
    let request = match request {
        Ok(request) => request,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };
    let reply = probe.send(request).await;
    let reply = match reply {
        Ok(reply) => reply,
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string()),
    };
    let reply = ron::to_string(&reply);
    let reply = match reply {
        Ok(reply) => reply,
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string()),
    };
    HttpResponse::Ok().body(reply)
}

#[post("/query")]
async fn query(req: String) -> impl Responder {
    let request = ron::from_str::<QueryMessage>(&req);
    let request = match request {
        Ok(request) => request,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };
    let reply = handle_query(request);
    let reply = match reply {
        Ok(reply) => reply,
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string()),
    };
    HttpResponse::Ok().body(reply)
}

async fn index() -> impl Responder {
    HttpResponse::Ok()
        .insert_header(header::ContentType(mime::TEXT_HTML))
        .body(asset::get("/index.html"))
}

pub async fn static_files(req: HttpRequest) -> HttpResponse {
    let filename: &str = req.match_info().query("filename");
    if !asset::contains(filename) {
        return HttpResponse::NotFound().body("");
    }
    let file = asset::get(filename);
    let mime_header = match filename {
        p if p.ends_with(".html") => header::ContentType(mime::TEXT_HTML),
        p if p.ends_with(".js") => header::ContentType(mime::APPLICATION_JAVASCRIPT),
        p if p.ends_with(".css") => header::ContentType(mime::TEXT_CSS),
        p if p.ends_with(".svg") => header::ContentType(mime::IMAGE_SVG),
        p if p.ends_with(".wasm") => header::ContentType(mime::APPLICATION_OCTET_STREAM),
        _ => header::ContentType(mime::TEXT_HTML),
    };
    HttpResponse::Ok().insert_header(mime_header).body(file)
}

pub fn page_service_config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/").route(web::get().to(index)))
        .service(web::resource("/overview").route(web::get().to(index)))
        .service(web::resource("/cluster").route(web::get().to(index)))
        .service(web::resource("/activity").route(web::get().to(index)))
        .service(web::resource("/inspect").route(web::get().to(index)))
        .service(web::resource("/index.html").route(web::get().to(index)));
}

pub fn api_service_config(cfg: &mut web::ServiceConfig) {
    cfg.service(probe).service(query);
}
