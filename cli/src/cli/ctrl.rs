use anyhow::Result;

use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioIo;
use hyperparameter::*;

use crate::inject::Process;

use super::{send_ctrl_via_ptrace, send_ctrl_via_socket};

#[derive(Clone)]
pub enum CtrlChannel {
    Ptrace { pid: i32 },
    Local { pid: i32 },
    Remote { addr: String },
}

impl TryFrom<&str> for CtrlChannel {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        if let [_, _] = value.split(':').collect::<Vec<_>>()[..] {
            return Ok(Self::Remote { addr: value.into() });
        }

        let callback = |pid| -> Result<CtrlChannel> {
            with_params! {
                get use_ptrace = probing.cli.ptrace or false;

                Ok(if use_ptrace {Self::Ptrace { pid }} else {Self::Local { pid }})
            }
        };

        if let Ok(pid) = value.parse::<i32>() {
            return callback(pid);
        }

        let pid = Process::by_cmdline(value).map_err(|err| {
            anyhow::anyhow!("failed to find process with cmdline pattern {value}: {err}")
        })?;
        if let Some(pid) = pid {
            return callback(pid);
        } else {
            return Err(anyhow::anyhow!("either `pid` or `name` must be specified"));
        }
    }
}

impl TryFrom<String> for CtrlChannel {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

impl Into<String> for CtrlChannel {
    fn into(self) -> String {
        match self {
            CtrlChannel::Ptrace { pid } | CtrlChannel::Local { pid } => format! {"{pid}"},
            CtrlChannel::Remote { addr } => format!("{addr}"),
        }
    }
}

impl CtrlChannel {
    pub fn send_ctrl(&self, cmd: String) -> Result<()> {
        match self {
            CtrlChannel::Ptrace { pid } => send_ctrl_via_ptrace(cmd, *pid),
            CtrlChannel::Local { pid } => send_ctrl_via_socket(cmd, *pid),
            CtrlChannel::Remote { addr } => todo!(),
        }
    }
}

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
    let request = if let Some(body) = body {
        Request::builder()
            .method("POST")
            .uri(url)
            .body(Full::<Bytes>::from(body))
            .unwrap()
    } else {
        Request::builder()
            .method("GET")
            .uri(url)
            .body(Full::<Bytes>::default())
            .unwrap()
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
