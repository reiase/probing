use std::marker::PhantomData;

use anyhow::Result;
use local_ip_address::*;
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
            use Color::{Blue, Green, Red};

            eprintln!("{}", Red.bold().paint("probe server is available on:"));
            if addr.to_string().starts_with("0.0.0.0:") {
                for (_, ip) in list_afinet_netifas()
                    .unwrap()
                    .iter()
                    .filter(|(_, ip)| ip.is_ipv4())
                {
                    let if_addr = ip.to_string();
                    let if_addr = addr.to_string().replace("0.0.0.0", &if_addr);
                    eprintln!("\t{}", Blue.bold().underline().paint(if_addr));
                }

                let local_addr = local_ip().unwrap().to_string();
                Some(addr.to_string().replace("0.0.0.0", &local_addr))
            } else {
                eprintln!("\t{}", Green.bold().underline().paint(addr.to_string()));
                Some(addr.to_string())
            }
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
