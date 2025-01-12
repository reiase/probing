use actix_web::{get, put, web, HttpResponse, Responder};
use anyhow::Result;

use probing_proto::{prelude::*, Process};

pub fn overview() -> Result<Process> {
    let current = procfs::process::Process::myself()?;
    let info = Process {
        pid: current.pid(),
        exe: current
            .exe()
            .map(|exe| exe.to_string_lossy().to_string())
            .unwrap_or("nil".to_string()),
        env: current
            .environ()
            .map(|m| {
                let envs: Vec<String> = m
                    .iter()
                    .map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy()))
                    .collect();
                envs.join("\n")
            })
            .unwrap_or("".to_string()),
        cmd: current
            .cmdline()
            .map(|cmds| cmds.join(" "))
            .unwrap_or("".to_string()),
        cwd: current
            .cwd()
            .map(|cwd| cwd.to_string_lossy().to_string())
            .unwrap_or("".to_string()),
        main_thread: current
            .task_main_thread()
            .map(|p| p.pid as u64)
            .unwrap_or(0),
        threads: current
            .tasks()
            .map(|iter| iter.map(|r| r.map(|p| p.tid as u64).unwrap_or(0)).collect())
            .unwrap_or_default(),
    };
    Ok(info)
}

#[get("/overview")]
async fn api_get_overview() -> impl Responder {
    match overview() {
        Ok(info) => HttpResponse::Ok().json(info),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    }
}

#[derive(serde::Deserialize)]
struct Info {
    tid: Option<i32>,
    path: Option<String>,
}

#[get("/callstack")]
async fn api_get_callstack(req: web::Query<Info>) -> impl Responder {
    let tid: Option<i32> = req.tid;
    let probe = crate::server::services::PROBE.clone();

    let reply = match probe.send(ProbeCall::CallBacktrace(tid)).await {
        Ok(reply) => reply,
        Err(err) => ProbeCall::Err(err.to_string()),
    };
    let reply = match ron::to_string(&reply) {
        Ok(reply) => reply,
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string()),
    };
    HttpResponse::Ok().body(reply)
}

#[get("/files")]
async fn api_get_files(req: web::Query<Info>) -> impl Responder {
    let content = if let Some(path) = req.path.clone() {
        std::fs::read_to_string(path).unwrap_or_default()
    } else {
        "".to_string()
    };
    HttpResponse::Ok().body(content)
}

#[put("/nodes")]
async fn put_nodes(req: String) -> impl Responder {
    let request = serde_json::from_str::<Node>(&req);
    let request = match request {
        Ok(request) => request,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };
    use probing_engine::plugins::cluster::service::update_node;
    update_node(request);
    HttpResponse::Ok().body("")
}

#[get("/nodes")]
async fn get_nodes() -> impl Responder {
    use probing_engine::plugins::cluster::service::get_nodes;
    let nodes = get_nodes();
    let nodes = serde_json::to_string(&nodes);
    let nodes = match nodes {
        Ok(nodes) => nodes,
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string()),
    };
    HttpResponse::Ok().body(nodes)
}

pub fn api_service_config(cfg: &mut web::ServiceConfig) {
    cfg.service(put_nodes)
        .service(get_nodes)
        .service(api_get_overview)
        .service(api_get_callstack);
}
