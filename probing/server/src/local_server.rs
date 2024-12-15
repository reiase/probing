use std::sync::Arc;
use std::thread;

use anyhow::Result;
use tokio::net::UnixListener;
use tokio::net::UnixStream;

use super::stream_handler::StreamHandler;
use probing_core::ProbeFactory;

pub struct LocalServer {
    acceptor: UnixListener,
    probe_factory: Arc<dyn ProbeFactory>,
}

unsafe impl Send for LocalServer {}

impl LocalServer {
    pub fn new(acceptor: UnixListener, probe_factory: Arc<dyn ProbeFactory>) -> Self {
        Self {
            acceptor,
            probe_factory,
        }
    }

    async fn run(&mut self) -> Result<()> {
        loop {
            let (stream, _) = self.acceptor.accept().await?;
            let probe = self.probe_factory.create();
            tokio::spawn(
                async move { StreamHandler::<UnixStream>::new(stream, probe).run().await },
            );
        }
    }
}

async fn local_server_worker(probe_factory: Arc<dyn ProbeFactory>) -> Result<()> {
    let prefix = std::env::var("PROBING_CTRL_ROOT").unwrap_or("/tmp/probing/".to_string());

    let path = std::path::Path::new(&prefix);
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }

    let path = format!("{}/{}", prefix, std::process::id());
    let path = std::path::Path::new(&path);
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    let mut server = LocalServer::new(UnixListener::bind(path)?, probe_factory);
    server.run().await
}

pub fn start(probe_factory: Arc<dyn ProbeFactory>) {
    thread::spawn(move || {
        let _ = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(local_server_worker(probe_factory));
    });
}

pub fn stop() -> Result<()> {
    let prefix = std::env::var("PROBING_CTRL_ROOT").unwrap_or("/tmp/probing/".to_string());

    let pid = std::process::id();
    let path = format!("{}/{}", prefix, pid);
    let path = std::path::Path::new(&path);
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    Ok(())
}
