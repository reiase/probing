use anyhow::Result;

use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioIo;
use probing_proto::prelude::ProbeCall;
use probing_proto::protocol::query::Query;

use crate::table::render_dataframe;

use probing_proto::prelude::*;

pub fn probe(ctrl: ProbeEndpoint, cmd: ProbeCall) -> Result<()> {
    let reply = ctrl.probe(cmd)?;
    println!("{reply}");
    Ok(())
}

pub fn query(ctrl: ProbeEndpoint, query: Query) -> Result<()> {
    let reply = ctrl.query(QueryMessage::Query(query))?;
    render_dataframe(&reply);
    Ok(())
}

#[derive(Clone)]
pub enum ProbeEndpoint {
    Ptrace { pid: i32 },
    Local { pid: i32 },
    Remote { addr: String },
    Launch { cmd: String },
}

impl TryFrom<&str> for ProbeEndpoint {
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

impl TryFrom<String> for ProbeEndpoint {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.as_str().try_into()
    }
}

impl From<ProbeEndpoint> for String {
    fn from(val: ProbeEndpoint) -> Self {
        match val {
            ProbeEndpoint::Ptrace { pid } | ProbeEndpoint::Local { pid } => format! {"{pid}"},
            ProbeEndpoint::Remote { addr } => addr,
            ProbeEndpoint::Launch { cmd } => cmd,
        }
    }
}

impl ProbeEndpoint {
    fn run_in_runtime<F, T>(&self, fut: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(fut)
    }

    fn send_request(&self, url: &str, body: &str) -> Result<String> {
        let bytes = self.run_in_runtime(request(self.clone(), url, Some(body.to_string())))?;
        Ok(String::from_utf8(bytes)?)
    }

    pub fn probe(&self, cmd: ProbeCall) -> Result<ProbeCall> {
        let cmd_str = ron::to_string(&cmd)?;
        let reply = self.send_request("/probe", &cmd_str)?;
        Ok(ron::from_str::<ProbeCall>(&reply)?)
    }

    pub fn query(&self, q: QueryMessage) -> Result<DataFrame> {
        let q_str = ron::to_string(&q)?;
        let reply = self.send_request("/query", &q_str)?;
        let reply = ron::from_str::<QueryMessage>(&reply)?;

        if let QueryMessage::Reply(r) = reply {
            let df = match r.format {
                QueryDataFormat::JSON => serde_json::from_slice(&r.data)?,
                QueryDataFormat::RON => ron::from_str(&String::from_utf8(r.data)?)?,
                _ => todo!(),
            };
            Ok(df)
        } else {
            Err(anyhow::anyhow!("unexpected reply: {:?}", reply))
        }
    }
}

pub async fn request(ctrl: ProbeEndpoint, url: &str, body: Option<String>) -> Result<Vec<u8>> {
    use hyper::body::Bytes;
    use hyper::client::conn;
    use hyper::Request;

    let mut sender = match ctrl {
        ProbeEndpoint::Ptrace { pid } | ProbeEndpoint::Local { pid } => {
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
        ProbeEndpoint::Remote { addr } => {
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
