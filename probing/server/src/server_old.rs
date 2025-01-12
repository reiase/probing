use std::sync::Arc;
use std::thread;

use anyhow::Result;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::net::TcpListener;
use tokio::net::UnixListener;
use tokio::net::UnixStream;

use hyper::server::conn::http1;
use hyper::service::service_fn;

use crate::handler::handle_request;

use super::tokio_io::TokioIo;
use probing_proto::protocol::probe::Probe;
use probing_proto::protocol::probe::ProbeFactory;

trait Acceptor: Send + Sync + 'static {
    type Stream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin + 'static;

    async fn accept(&self) -> Result<(Self::Stream, String)>;
    fn addr(&self) -> String;
}

impl Acceptor for UnixListener {
    type Stream = UnixStream;

    async fn accept(&self) -> Result<(Self::Stream, String)> {
        let (stream, _) = self.accept().await?;
        Ok((stream, self.addr()))
    }
    fn addr(&self) -> String {
        if let Ok(addr) = self.local_addr() {
            addr.as_pathname()
                .map(|addr| addr.to_string_lossy().to_string())
                .unwrap_or_else(|| "unix://".to_string())
        } else {
            "unix://".to_string()
        }
    }
}

impl Acceptor for TcpListener {
    type Stream = tokio::net::TcpStream;

    async fn accept(&self) -> Result<(Self::Stream, String)> {
        let stream = TcpListener::accept(self).await?;
        Ok((stream.0, stream.1.to_string()))
    }

    fn addr(&self) -> String {
        self.local_addr()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|_| "tcp://".to_string())
    }
}

struct Server<A: Acceptor> {
    acceptor: Box<A>,
    probe_factory: Arc<dyn ProbeFactory>,
}

impl<A: Acceptor> Server<A> {
    pub fn new(acceptor: Box<A>, probe_factory: Arc<dyn ProbeFactory>) -> Self {
        Self {
            acceptor,
            probe_factory,
        }
    }

    async fn handle_connection<IO>(stream: IO, probe: Arc<dyn Probe>) -> Result<()>
    where
        IO: AsyncRead + AsyncWrite + std::marker::Unpin,
    {
        http1::Builder::new()
            .serve_connection(
                TokioIo::new(stream),
                service_fn(|request| {
                    let probe = probe.clone();
                    async move {
                        handle_request(request, probe).await.map_err(|err| {
                            log::error!("error when handling probe request: {}", err);
                            err
                        })
                    }
                }),
            )
            .await
            .map_err(|err| err.into())
    }

    async fn run(&mut self) -> Result<()> {
        log::info!("Server listening on {}", self.acceptor.addr());
        loop {
            let (stream, peer_addr) = self.acceptor.accept().await?;
            log::debug!("New connection from {}", peer_addr);

            let probe = self.probe_factory.create();
            tokio::spawn(async move { Self::handle_connection(stream, probe).await });
        }
    }
}

impl Server<UnixListener> {
    async fn local(probe_factory: Arc<dyn ProbeFactory>) -> Result<Self> {
        let socket_path = std::env::var("PROBING_CTRL_ROOT").unwrap_or("/tmp/probing/".to_string());
        let socket_path = format!("{}/{}", socket_path, std::process::id());

        if let Some(parent) = std::path::Path::new(&socket_path).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let _ = tokio::fs::remove_file(&socket_path).await;

        let acceptor = Box::new(UnixListener::bind(&socket_path)?);
        Ok(Server::new(acceptor, probe_factory))
    }
}

impl Server<TcpListener> {
    async fn remote(addr: String, probe_factory: Arc<dyn ProbeFactory>) -> Result<Self> {
        let reslved_addr = if let Some((host, port)) = addr.split_once(":") {
            let ip = dns_lookup::lookup_host(host).map_err(|e| {
                println!("resolve {} failed: {}", host, e);
                e
            })?;
            let ip = ip
                .iter()
                .filter(|ipaddr| ipaddr.is_ipv4())
                .collect::<Vec<_>>();
            let reslved_addr = format!("{}:{}", ip[0], port);
            println!("resolve {} to {}", addr, reslved_addr);
            reslved_addr
        } else {
            addr
        };
        let acceptor = Box::new(TcpListener::bind(reslved_addr).await?);

        // Properly set the PROBING_ADDRESS and print the server address
        if let Ok(addr) = acceptor.local_addr() {
            {
                let mut probing_address = crate::vars::PROBING_ADDRESS.write().unwrap();
                *probing_address = addr.to_string();
            }
            use nu_ansi_term::Color::{Green, Red};

            eprintln!("{}", Red.bold().paint("probing server is available on:"));
            eprintln!("\t{}", Green.bold().underline().paint(addr.to_string()));
        }

        Ok(Server::new(acceptor, probe_factory))
    }
}

pub fn start_local(probe_factory: Arc<dyn ProbeFactory>) {
    thread::spawn(move || {
        let _ = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let mut server = Server::local(probe_factory).await?;
                server.run().await
            });
    });
}

pub fn start_remote(addr: Option<String>, probe_factory: Arc<dyn ProbeFactory>) {
    let addr = addr.unwrap_or_else(|| "0.0.0.0:0".to_string());
    thread::spawn(move || {
        let _ = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let mut server = Server::remote(addr, probe_factory).await?;
                server.run().await
            });
    });
}

pub fn cleanup() -> Result<()> {
    let prefix = std::env::var("PROBING_CTRL_ROOT").unwrap_or("/tmp/probing/".to_string());

    let pid = std::process::id();
    let path = format!("{}/{}", prefix, pid);
    let path = std::path::Path::new(&path);
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    Ok(())
}
