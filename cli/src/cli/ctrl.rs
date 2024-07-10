use anyhow::Result;

use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioIo;

pub async fn request(pid: i32, url: &str, body: Option<String>) -> Result<String> {
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
    let request = Request::builder().method("GET").uri(&format!("/{}", url));
    let request = if let Some(body) = body {
        request.body(Full::<Bytes>::from(body)).unwrap()
    } else {
        request.body(Full::<Bytes>::default()).unwrap()
    };

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

    if res.status().is_success() {
        Ok(body)
    } else {
        anyhow::bail!("Error {}: {}", res.status(), body)
    }
}
