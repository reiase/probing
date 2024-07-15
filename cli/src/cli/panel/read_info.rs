use anyhow::Result;

use http_body_util::BodyExt;
use hyper_util::rt::TokioIo;

use hyperparameter::*;
use probing_common::{CallStack, Object};

use crate::cli::ctrl::CtrlChannel;

async fn request(ctrl: CtrlChannel, url: &str) -> Result<String> {
    use http_body_util::Empty;
    use hyper::body::Bytes;
    use hyper::client::conn;
    use hyper::Request;

    let mut sender = match ctrl {
        CtrlChannel::Ptrace { pid } | CtrlChannel::Local { pid } => {
            let prefix = "/tmp/probing".to_string();
            let path = format!("{}/{}", prefix, pid);
            let path = std::path::Path::new(&path);
            if !path.exists() {
                anyhow::bail!("server not found: {}", path.display());
            }
            let stream = tokio::net::UnixStream::connect(path).await?;
            let io = TokioIo::new(stream);

            let (sender, connection) = conn::http1::handshake(io).await?;
            tokio::spawn(async move {
                connection.await.unwrap();
            });
            sender
        }
        CtrlChannel::Remote { addr } => {
            let stream = tokio::net::TcpStream::connect(addr).await?;
            let io = TokioIo::new(stream);

            let (sender, connection) = conn::http1::handshake(io).await?;
            tokio::spawn(async move {
                connection.await.unwrap();
            });
            sender
        }
    };

    let request = Request::builder()
        .method("GET")
        .uri(&format!("/{}", url))
        .body(Empty::<Bytes>::new())
        .unwrap();
    let res = sender.send_request(request).await.unwrap();
    let ret = res.into_body().collect().await?.to_bytes().to_vec();
    let body = String::from_utf8(ret).unwrap();
    Ok(body)
}

pub fn read_process_info() -> String {
    let mut process_info = Default::default();
    with_params! {
        get ctrl = probing.ctrl.uri or "".to_string();

        let ctrl: CtrlChannel = ctrl.try_into().unwrap();
        process_info = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(request(ctrl, "apis/overview"))
            .unwrap();
    }

    process_info
}

pub fn read_callstack_info(tid: i32) -> Result<Vec<CallStack>> {
    let mut ret: Vec<CallStack> = vec![];
    with_params! {
        get ctrl = probing.ctrl.uri or "".to_string();

        let ctrl: CtrlChannel = ctrl.try_into().unwrap();
        let url = format!("apis/callstack?tid={}", tid);
        let info = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(request(ctrl,  url.as_str()))?;
        ret = serde_json::from_str(info.as_str())?;
    }

    Ok(ret)
}

pub fn read_object_info(url: &str) -> Result<Vec<Object>> {
    let mut ret: Vec<Object> = vec![];
    with_params! {
        get ctrl = probing.ctrl.uri or "".to_string();

        let ctrl: CtrlChannel = ctrl.try_into().unwrap();
        let info = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(request(ctrl,  url))?;

        ret = serde_json::from_str(info.as_str())?;
    };

    Ok(ret)
}
