use anyhow::Result;

use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioIo;
use probing_proto::prelude::ProbeCall;
use probing_proto::protocol::query::Query;

use crate::table::render_dataframe;
use probing_proto::cli::CtrlSignal;

use probing_proto::prelude::*;

pub fn probe(ctrl: CtrlChannel, cmd: ProbeCall) -> Result<()> {
    let reply = ctrl.probe(cmd)?;
    println!("{reply}");
    Ok(())
}

pub fn handle(ctrl: CtrlChannel, sig: CtrlSignal) -> Result<()> {
    let cmd = ron::to_string(&sig)?;
    match ctrl.execute(cmd) {
        Ok(ret) => {
            let ret = String::from_utf8(ret)?;
            println!("{ret}");
        }
        Err(err) => println!("{err}"),
    }
    Ok(())
}

pub fn query(ctrl: CtrlChannel, query: Query) -> Result<()> {
    let reply = ctrl.query(QueryMessage::Query(query))?;
    render_dataframe(&reply);
    Ok(())
}

#[derive(Clone)]
pub enum CtrlChannel {
    Ptrace { pid: i32 },
    Local { pid: i32 },
    Remote { addr: String },
    Launch { cmd: String },
}

impl TryFrom<&str> for CtrlChannel {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        if let [_, _] = value.split(':').collect::<Vec<_>>()[..] {
            return Ok(Self::Remote { addr: value.into() });
        }

        Ok(Self::Local {
            pid: value.parse::<i32>()?,
        })
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
            CtrlChannel::Launch { cmd } => cmd,
        }
    }
}

impl CtrlChannel {
    pub fn execute(&self, cmd: String) -> Result<Vec<u8>> {
        match self {
            ctrl => {
                let ret = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(request(ctrl.clone(), "/ctrl", cmd.into()))?;

                Ok(ret)
            }
            _ => todo!(),
        }
    }

    pub fn probe(&self, cmd: ProbeCall) -> Result<ProbeCall> {
        let cmd = ron::to_string(&cmd)?;
        log::debug!("request: {cmd}");
        match self {
            // CtrlChannel::Ptrace { pid } => {
            //     send_ctrl_via_ptrace(cmd, *pid)?;
            //     Ok(ProbeCall::Nil)
            // }
            ctrl => {
                let reply = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(request(ctrl.clone(), "/probe", cmd.into()))?;

                let reply = String::from_utf8(reply)?;
                log::debug!("reply: {reply}");
                let reply = ron::from_str::<ProbeCall>(reply.as_str())?;

                Ok(reply)
            }
            _ => todo!(),
        }
    }

    pub fn query(&self, req: QueryMessage) -> Result<DataFrame> {
        let req = ron::to_string(&req)?;
        log::debug!("request: {req}");
        let ctrl = self;
        {
            let reply = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(request(ctrl.clone(), "/query", req.into()))?;

            let reply = String::from_utf8(reply)?;
            log::debug!("reply: {reply}");
            let reply = ron::from_str::<QueryMessage>(reply.as_str())?;

            if let QueryMessage::Reply(reply) = reply {
                let df: DataFrame = match reply.format {
                    QueryDataFormat::JSON => serde_json::from_slice(&reply.data)?,
                    QueryDataFormat::RON => ron::from_str(String::from_utf8(reply.data)?.as_str())?,
                    _ => todo!(),
                };
                return Ok(df);
            }
            Err(anyhow::anyhow!("unexpected reply: {:?}", reply))
        }
    }

    pub fn signal(&self, cmd: String) -> Result<()> {
        match self {
            // CtrlChannel::Ptrace { pid } => {
            //     send_ctrl_via_ptrace(cmd, *pid)?;
            //     Ok(())
            // }
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
            _ => todo!(),
        }
    }
}

pub async fn request(ctrl: CtrlChannel, url: &str, body: Option<String>) -> Result<Vec<u8>> {
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
        _ => todo!(),
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

    Ok(res.collect().await.map(|x| x.to_bytes().to_vec())?)
}

// fn send_ctrl_via_ptrace(argstr: String, pid: i32) -> Result<()> {
//     eprintln!("sending ctrl commands via ptrace...");
//     let process = Process::get(pid as u32).unwrap();
//     Injector::attach(process)
//         .unwrap()
//         .setenv(Some("PROBING_ARGS"), Some(argstr.as_str()))
//         .map_err(|e| anyhow::anyhow!(e))?;
//     signal::kill(Pid::from_raw(pid), signal::Signal::SIGUSR1)?;
//     Ok(())
// }
