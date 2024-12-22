use std::marker::PhantomData;
use std::thread;

use anyhow::Result;
// use hyperparameter::*;
use tokio::net::UnixListener;
use tokio::net::UnixStream;

use super::stream_handler::StreamHandler;
use crate::repl::Repl;

pub struct LocalServer<T> {
    acceptor: UnixListener,
    phantom: PhantomData<T>,
}

unsafe impl<T> Send for LocalServer<T> {}

impl<T> LocalServer<T>
where
    T: Repl + Default + Send,
{
    pub fn create(acceptor: UnixListener) -> Self {
        Self {
            acceptor,
            phantom: PhantomData,
        }
    }

    async fn run(&mut self) -> Result<()> {
        loop {
            let (stream, _) = self.acceptor.accept().await?;
            tokio::spawn(async move { StreamHandler::<UnixStream, T>::new(stream).run().await });
        }
    }
}

async fn local_server_worker<T>() -> Result<()>
where
    T: Repl + Default + Send,
{
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

        let mut server = LocalServer::<T>::create(UnixListener::bind(path)?);
        server.run().await
    }
}

pub fn start<T>()
where
    T: Repl + Default + Send,
{
    thread::spawn(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(local_server_worker::<T>())
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
