use std::{marker::PhantomData, thread};

use anyhow::Result;
use log::debug;
use nu_ansi_term::Color;
use tokio::net::{TcpListener, TcpStream};

use super::stream_handler::StreamHandler;
use crate::{repl::Repl, server::vars::PROBING_ADDRESS};

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
            {
                let mut probing_address = PROBING_ADDRESS.write().unwrap();
                *probing_address = addr.to_string();
            }
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

pub async fn remote_server_worker<T>(addr: Option<String>) -> Result<()>
where
    T: Repl + Default + Send,
{
    let mut server = match addr {
        Some(addr) => AsyncServer::<T>::new(addr),
        None => AsyncServer::<T>::default(),
    };
    server.run().await
}

pub fn start<T>(addr: Option<String>)
where
    T: Repl + Default + Send,
{
    thread::spawn(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(remote_server_worker::<T>(addr))
            .unwrap();
    });
}
