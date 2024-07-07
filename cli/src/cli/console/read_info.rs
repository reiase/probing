use anyhow::Result;

use http_body_util::BodyExt;
use hyper_util::rt::TokioIo;

use hyperparameter::*;

async fn request(pid: i32, url: &str) -> Result<String> {
    use http_body_util::Empty;
    use hyper::body::Bytes;
    use hyper::client::conn;
    use hyper::Request;

    let prefix = "/tmp/probing/".to_string();
    let path = format!("{}/{}", prefix, pid);
    let path = std::path::Path::new(&path);
    if !path.exists() {
        anyhow::bail!("server not found");
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
    let mut res = sender.send_request(request).await.unwrap();
    let mut ret: Vec<u8> = vec![];

    while let Some(next) = res.frame().await {
        if let Ok(frame) = next {
            if let Some(chunk) = frame.data_ref() {
                ret.extend_from_slice(chunk);
            }
        }
    }
    let body = String::from_utf8(ret).unwrap();
    return Ok(body);
}

pub fn read_process_info() -> String {
    let mut process_info = "".to_string();
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
