use std::marker::PhantomData;

use anyhow::Result;
use log::debug;
use nu_ansi_term::Color;
use tokio::net::{TcpListener, TcpStream};

use super::stream_handler::StreamHandler;
use crate::repl::Repl;

pub struct AsyncServer<T> {
    self_addr: Option<String>,
    phantom: PhantomData<T>,
}

unsafe impl<T> Send for AsyncServer<T> {}

impl<T> Default for AsyncServer<T> {
    fn default() -> Self {
        Self {
            self_addr: Some("0.0.0.0:0".to_string()),
            phantom: PhantomData,
        }
    }
}

impl<T: Repl + Default + Send> AsyncServer<T> {
    pub fn new(addr: String) -> Self {
        Self {
            self_addr: Some(addr),
            phantom: PhantomData,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let listener = TcpListener::bind(self.self_addr.as_ref().unwrap()).await?;
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
            tokio::spawn(async move { StreamHandler::<TcpStream, T>::new(stream).run().await });
        }
    }
}

pub async fn start_async_server<T>(addr: Option<String>) -> Result<()>
where
    T: Repl + Default + Send,
{
    let mut server = match addr {
        Some(addr) => AsyncServer::<T>::new(addr),
        None => AsyncServer::<T>::default(),
    };
    server.run().await
}
