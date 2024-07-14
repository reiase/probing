use anyhow::Result;

use http_body_util::BodyExt;
use hyper_util::rt::TokioIo;

use hyperparameter::*;
use probing_common::{CallStack, Object};

async fn request(pid: i32, url: &str) -> Result<String> {
    use http_body_util::Empty;
    use hyper::body::Bytes;
    use hyper::client::conn;
    use hyper::Request;

    let prefix = "/tmp/probing".to_string();
    let path = format!("{}/{}", prefix, pid);
    let path = std::path::Path::new(&path);
    if !path.exists() {
        anyhow::bail!("server not found: {}", path.display());
    }
    let stream = tokio::net::UnixStream::connect(path).await?;
    let io = TokioIo::new(stream);

    let (mut sender, connection) = conn::http1::handshake(io).await?;
    tokio::spawn(async move {
        connection.await.unwrap();
    });
    let request = Request::builder()
        .method("GET")
        .uri(&format!("/{}", url))
        .body(Empty::<Bytes>::new())
        .unwrap();
    let res = sender.send_request(request).await.unwrap();
    let ret = res.into_body().collect().await?.to_bytes().to_vec();
    let body = String::from_utf8(ret).unwrap();
    return Ok(body);
}

pub fn read_process_info() -> String {
    let mut process_info = Default::default();
    with_params! {
        get pid = probing.process.pid or 0;

        process_info = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(request(pid as i32, "apis/overview"))
            .unwrap();
    }

    process_info
}

pub fn read_callstack_info(tid: i32) -> Result<Vec<CallStack>> {
    let mut ret: Vec<CallStack> = vec![];
    with_params! {
        get pid = probing.process.pid or 0;

        let url = format!("apis/callstack?tid={}", tid);
        let info = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(request(pid as i32,  url.as_str()))?;
        ret = serde_json::from_str(info.as_str())?;
    }

    Ok(ret)
}

pub fn read_object_info(url: &str) -> Result<Vec<Object>> {
    let mut ret: Vec<Object> = vec![];
    with_params! {
        get pid = probing.process.pid or 0;

        let info = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(request(pid as i32,  url))?;

        ret = serde_json::from_str(info.as_str())?;
    };

    Ok(ret)
}
