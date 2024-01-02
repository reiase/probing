use nu_ansi_term::Color;
use std::{error::Error, marker::PhantomData, thread::sleep, time, u8};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::repl::REPL;
use local_ip_address::list_afinet_netifas;
use local_ip_address::local_ip;

async fn show_prompt(prompt: &[u8], stream: &mut TcpStream) {
    let mut peek_buf = [0u8; 4];
    sleep(time::Duration::from_millis(10));
    let ret = stream.peek(&mut peek_buf).await;
    match ret {
        Ok(ulen) if ulen == 4 => {
            if !peek_buf.starts_with("GET ".as_bytes()) {
                let _ = stream.write(prompt).await;
            }
        }
        _ => {
            let _ = stream.write(prompt).await;
        }
    };
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

impl<T: REPL + Default + Send> AsyncServer<T> {
    pub fn new(addr: String) -> Self {
        Self {
            self_addr: Some(addr),
            prompt: None,
            phantom: PhantomData,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(self.self_addr.as_ref().unwrap()).await?;
        self.self_addr = match listener.local_addr() {
            Ok(addr) => {
                if addr.to_string().contains("0.0.0.0:") {
                    println!(
                        "{} {}, available addresses:",
                        Color::Red.bold().paint("Debug Server is started on"),
                        Color::Green.bold().underline().paint(addr.to_string())
                    );
                    for (name, ip) in list_afinet_netifas().unwrap().iter() {
                        if !ip.is_ipv4() {
                            continue;
                        }
                        let if_addr = ip.to_string();
                        println!(
                            "\t{}: {}",
                            Color::Yellow.paint(name),
                            Color::Blue
                                .bold()
                                .underline()
                                .paint(addr.to_string().replace("0.0.0.0", &if_addr))
                        );
                    }

                    let local_addr = local_ip().unwrap().to_string();
                    Some(addr.to_string().replace("0.0.0.0", &local_addr))
                } else {
                    println!(
                        "{} {}",
                        Color::Red.bold().paint("Debug Server is started on"),
                        Color::Green.bold().underline().paint(addr.to_string())
                    );
                    Some(addr.to_string())
                }
            }
            Err(err) => {
                println!("error binding debug server address: {}", err.to_string());
                None
            }
        };
        self.prompt = self.self_addr.as_ref().map(|addr| format!("({})>>", addr));

        self.serve(&listener).await
    }

    async fn serve(&self, listener: &TcpListener) -> Result<(), Box<dyn Error>> {
        loop {
            let (mut stream, addr) = listener.accept().await?;
            let prompt = self.get_prompt().to_string();
            // Spawn our handler to be run asynchronously.
            tokio::spawn(async move {
                println!(
                    "{} {}",
                    Color::Yellow.italic().paint("debug server connection from"),
                    Color::Green.italic().underline().paint(addr.to_string())
                );
                let mut repl = Box::new(T::default());
                let mut buf = [0; 1024];
                // let _ = stream.write(prompt.as_bytes()).await;
                let _ = show_prompt(prompt.as_bytes(), &mut stream).await;
                loop {
                    let n = match stream.read(&mut buf).await {
                        Ok(n) if n == 0 => break,
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
            });
        }
    }

    fn get_prompt(&self) -> &str {
        self.prompt.as_ref().map_or(">>", |s| s.as_str())
    }
}

pub async fn start_async_server<T>(addr: Option<String>) -> Result<(), Box<dyn Error>>
where
    T: REPL + Default + Send,
{
    let mut server = match addr {
        Some(addr) => AsyncServer::<T>::new(addr),
        None => AsyncServer::<T>::default(),
    };
    server.run().await?;
    Ok(())
}
