use crate::{repl::Repl, server::service::TokioIo};
use hyper::server::conn::http1;
use local_ip_address::*;
use nu_ansi_term::Color;
use std::{error::Error, marker::PhantomData, thread::sleep, time};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::service::ProbeService;

async fn is_http(stream: &mut TcpStream) -> bool {
    let mut peek_buf = [0u8; 4];
    sleep(time::Duration::from_millis(10));

    stream.peek(&mut peek_buf).await.ok().map_or(false, |ulen| {
        ulen == 4
            && (peek_buf.starts_with("GET ".as_bytes())
                || peek_buf.starts_with("POST".as_bytes())
                || peek_buf.starts_with("OPTI".as_bytes()))
    })
}

pub struct AsyncServer<T> {
    self_addr: Option<String>,
    prompt: Option<String>,
    phantom: PhantomData<T>,
}

unsafe impl<T> Send for AsyncServer<T> {}

impl<T> Default for AsyncServer<T> {
    fn default() -> Self {
        Self {
            self_addr: Some("0.0.0.0:0".to_string()),
            prompt: None,
            phantom: PhantomData,
        }
    }
}

impl<T: Repl + Default + Send> AsyncServer<T> {
    pub fn new(addr: String) -> Self {
        Self {
            self_addr: Some(addr),
            prompt: None,
            phantom: PhantomData,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(self.self_addr.as_ref().unwrap()).await?;
        if let Ok(addr) = listener.local_addr() {
            eprintln!(
                "{}",
                Color::Red.bold().paint("probe server is available on:")
            );
            if addr.to_string().starts_with("0.0.0.0:") {
                for (_, ip) in list_afinet_netifas().unwrap().iter() {
                    if !ip.is_ipv4() {
                        continue;
                    }
                    let if_addr = ip.to_string();
                    let if_addr = addr.to_string().replace("0.0.0.0", &if_addr);
                    eprintln!("\t{}", Color::Blue.bold().underline().paint(if_addr));
                }

                let local_addr = local_ip().unwrap().to_string();
                Some(addr.to_string().replace("0.0.0.0", &local_addr))
            } else {
                eprintln!(
                    "\t{}",
                    Color::Green.bold().underline().paint(addr.to_string())
                );
                Some(addr.to_string())
            }
        } else {
            None
        };
        self.prompt = self.self_addr.as_ref().map(|addr| format!("({})>>", addr));
        self.serve(&listener).await
    }

    async fn serve(&self, listener: &TcpListener) -> Result<(), Box<dyn Error>> {
        loop {
            let (mut stream, addr) = listener.accept().await?;
            stream.nodelay().unwrap();
            let prompt = self.get_prompt().to_string();
            // Spawn our handler to be run asynchronously.
            tokio::spawn(async move {
                if is_http(&mut stream).await {
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), ProbeService::default())
                        .with_upgrades()
                        .await
                    {
                        println!("Failed to serve connection: {}\n{:?}", addr, err);
                    }
                } else {
                    eprintln!(
                        "{} {}",
                        Color::Yellow.italic().paint("debug server connection from"),
                        Color::Green.italic().underline().paint(addr.to_string())
                    );
                    let mut repl = Box::<T>::default();
                    let mut buf = [0; 1024];
                    let _ = stream.write(prompt.as_bytes()).await;
                    loop {
                        let n = match stream.read(&mut buf).await {
                            Ok(0) => break,
                            Ok(n) => n,
                            Err(_) => break,
                        };
                        let req = String::from_utf8(buf[0..n].to_vec());
                        let s = match repl.feed(req.clone().unwrap()) {
                            Some(rsp) => format!("{}\n{}", rsp, prompt),
                            None => prompt.to_string(),
                        };
                        if stream.write(s.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                }
            });
        }
    }

    fn get_prompt(&self) -> &str {
        self.prompt.as_ref().map_or(">>", |s| s.as_str())
    }
}

pub async fn start_async_server<T>(addr: Option<String>) -> Result<(), Box<dyn Error>>
where
    T: Repl + Default + Send,
{
    let mut server = match addr {
        Some(addr) => AsyncServer::<T>::new(addr),
        None => AsyncServer::<T>::default(),
    };
    server.run().await?;
    Ok(())
}
