use anyhow::Result;

use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioIo;

use probing_proto::{prelude::*, protocol::process::CallFrame};

use crate::table::render_dataframe;

pub async fn query(ctrl: ProbeEndpoint, query: Query) -> Result<()> {
    let reply = ctrl.query(query).await?;
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
    async fn send_request(&self, url: &str, body: &str) -> Result<String> {
        // Await request directly
        let bytes = request(self.clone(), url, Some(body.to_string())).await?;
        Ok(String::from_utf8(bytes)?)
    }

    pub async fn backtrace(&self, tid: Option<i32>) -> Result<()> {
        let mut url = "/apis/pythonext/callstack".to_string();
        if let Some(tid) = tid {
            url = format!("/apis/pythonext/callstack?tid={tid}");
        }
        let reply = request(self.clone(), &url, None).await?;
        match serde_json::from_slice::<Vec<CallFrame>>(&reply) {
            Ok(msg) => {
                for f in msg {
                    println!("{f}")
                }
                Ok(())
            }
            Err(err) => Err(anyhow::anyhow!("error: {}", err)),
        }
    }

    pub async fn eval(&self, code: String) -> Result<()> {
        let reply = request(self.clone(), "/apis/pythonext/eval", Some(code)).await?;

        println!("{}", String::from_utf8(reply)?);

        Ok(())
    }

    pub async fn query(&self, q: Query) -> Result<DataFrame> {
        let request = Message::new(q);
        let q_str = serde_json::to_string(&request)?;
        let reply_str = self.send_request("/query", &q_str).await?; // Renamed reply variable
        let reply = serde_json::from_str::<Message<QueryDataFormat>>(&reply_str)?.payload;

        match reply {
            QueryDataFormat::Error(err) => Err(anyhow::anyhow!("error: {}", err)),
            QueryDataFormat::Nil => Ok(Default::default()),
            QueryDataFormat::DataFrame(df) => Ok(df),
            QueryDataFormat::TimeSeries(_) => todo!(),
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
            let prefix = "\0".to_string();
            let path = format!("{prefix}probing-{pid}");

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
                connection
                    .await
                    .map_err(|err| {
                        eprintln!("error: {err}");
                    })
                    .unwrap();
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
