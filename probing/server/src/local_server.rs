use std::sync::Arc;
use std::thread;

use anyhow::Result;
use hyperparameter::*;
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
    with_params! {
        get prefix = probing.server.unix_socket_path or "/tmp/probing/".to_string();

        let path = std::path::Path::new(&prefix);
        if !path.exists(){
            std::fs::create_dir_all(path)?;
        }

        let pid = std::process::id();
        let path = format!("{}/{}", prefix, pid);
        let path = std::path::Path::new(&path);
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let mut server = LocalServer::new(UnixListener::bind(path)?, probe_factory);
        server.run().await
    }
}

pub fn start(probe_factory: Arc<dyn ProbeFactory>) {
    thread::spawn(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(local_server_worker(probe_factory))
            .unwrap();
    });
}

pub fn stop() -> Result<()> {
    with_params! {
        get prefix = probing.server.unix_socket_path or "/tmp/probing/".to_string();

        let pid = std::process::id();
        let path = format!("{}/{}", prefix, pid);
        let path = std::path::Path::new(&path);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}
