use anyhow::Result;

use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioIo;
use hyperparameter::*;
use nix::{sys::signal, unistd::Pid};

use crate::inject::{Injector, Process};
use probing_common::cli::CtrlSignal;

pub fn handle(ctrl: CtrlChannel, sig: CtrlSignal) -> Result<()> {
    let cmd = ron::to_string(&sig)?;
    match ctrl.query(cmd) {
        Ok(ret) => println!("{ret}"),
        Err(err) => println!("{err}"),
    }
    Ok(())
}

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
            callback(pid)
        } else {
            Err(anyhow::anyhow!("either `pid` or `name` must be specified"))
        }
    }
}

impl TryFrom<String> for CtrlChannel {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

impl From<CtrlChannel> for String {
    fn from(val: CtrlChannel) -> Self {
        match val {
            CtrlChannel::Ptrace { pid } | CtrlChannel::Local { pid } => format! {"{pid}"},
            CtrlChannel::Remote { addr } => addr,
        }
    }
}

impl CtrlChannel {
    pub fn query(&self, cmd: String) -> Result<String> {
        match self {
            CtrlChannel::Ptrace { pid } => {
                send_ctrl_via_ptrace(cmd, *pid)?;
                Ok(Default::default())
            }
            ctrl => {
                let ret = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(request(ctrl.clone(), "/ctrl", cmd.into()))?;

                Ok(ret)
            }
        }
    }

    pub fn signal(&self, cmd: String) -> Result<()> {
        match self {
            CtrlChannel::Ptrace { pid } => {
                send_ctrl_via_ptrace(cmd, *pid)?;
                Ok(Default::default())
            }
            ctrl => {
                let cmd = if cmd.starts_with('[') {
                    cmd
                } else {
                    format!("[{}]", cmd)
                };
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(request(ctrl.clone(), "/ctrl", cmd.into()))?;

                Ok(())
            }
        }
    }
}

pub async fn request(ctrl: CtrlChannel, url: &str, body: Option<String>) -> Result<String> {
    use hyper::body::Bytes;
    use hyper::client::conn;
    use hyper::Request;

    let mut sender = match ctrl {
        CtrlChannel::Ptrace { pid } | CtrlChannel::Local { pid } => {
            eprintln!("sending ctrl commands via unix socket...");
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
            eprintln!("sending ctrl commands via tcp socket...");
            let stream = tokio::net::TcpStream::connect(addr).await?;
            let io = TokioIo::new(stream);

            let (sender, connection) = conn::http1::handshake(io).await?;
            tokio::spawn(async move {
                connection.await.unwrap();
            });
            sender
        }
    };
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

    let res = sender.send_request(request).await?;

    let ret = res.collect().await?;
    let ret = String::from_utf8(ret.to_bytes().to_vec())?;
    return Ok(ret);
}

fn send_ctrl_via_ptrace(argstr: String, pid: i32) -> Result<()> {
    eprintln!("sending ctrl commands via ptrace...");
    let process = Process::get(pid as u32).unwrap();
    Injector::attach(process)
        .unwrap()
        .setenv(Some("PROBING_ARGS"), Some(argstr.as_str()))
        .map_err(|e| anyhow::anyhow!(e))?;
    signal::kill(Pid::from_raw(pid), signal::Signal::SIGUSR1)?;
    Ok(())
}
