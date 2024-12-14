use std::{sync::Arc, thread};

use anyhow::Result;
use log::debug;
use nu_ansi_term::Color;
use tokio::net::{TcpListener, TcpStream};

use super::stream_handler::StreamHandler;
use probing_core::ProbeFactory;

pub struct AsyncServer {
    self_addr: Option<String>,
    probe_factory: Arc<dyn ProbeFactory>,
}

unsafe impl Send for AsyncServer {}

impl AsyncServer {
    pub fn new(addr: String, probe_factory: Arc<dyn ProbeFactory>) -> Self {
        Self {
            self_addr: Some(addr),
            probe_factory,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let listener = if let Some((host, port)) = self.self_addr.as_ref().unwrap().split_once(":")
        {
            let addr = dns_lookup::lookup_host(host).map_err(|e| {
                println!("resolve {} failed: {}", host, e);
                e
            })?;
            let addr = addr
                .iter()
                .filter(|ipaddr| ipaddr.is_ipv4())
                .collect::<Vec<_>>();
            let addr = format!("{}:{}", addr[0], port);
            println!("resolve {} to {}", self.self_addr.as_ref().unwrap(), addr);
            TcpListener::bind(addr).await?
        } else {
            TcpListener::bind(self.self_addr.as_ref().unwrap()).await?
        };
        if let Ok(addr) = listener.local_addr() {
            use Color::{Green, Red};

            eprintln!("{}", Red.bold().paint("probing server is available on:"));
            eprintln!("\t{}", Green.bold().underline().paint(addr.to_string()));
            Some(addr.to_string())
        } else {
            None
        };
        self.serve(&listener).await
    }

    async fn serve(&self, listener: &TcpListener) -> Result<()> {
        loop {
            let (stream, addr) = listener.accept().await?;

            stream.nodelay().unwrap();

            debug!("new connection from {}", addr);
            let probe = self.probe_factory.create();

            tokio::spawn(async move { StreamHandler::<TcpStream>::new(stream, probe).run().await });
        }
    }
}

pub async fn remote_server_worker(
    addr: Option<String>,
    probe_factory: Arc<dyn ProbeFactory>,
) -> Result<()> {
    let addr = addr.unwrap_or("0.0.0.0:0".to_string());
    let mut server = AsyncServer::new(addr, probe_factory);
    server.run().await
}

pub fn start(addr: Option<String>, probe_factory: Arc<dyn ProbeFactory>) {
    thread::spawn(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(remote_server_worker(addr, probe_factory))
            .unwrap();
    });
}
