use std::marker::PhantomData;

use anyhow::Result;
use hyper::server::conn::http1;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::UnixStream;

use super::tokio_io::TokioIo;
use crate::repl::Repl;
use crate::service::ProbingService;

pub trait IsHTTP {
    async fn is_http(&mut self) -> bool;
}

impl IsHTTP for TcpStream {
    async fn is_http(&mut self) -> bool {
        let mut peek_buf = [0u8; 4];
        std::thread::sleep(std::time::Duration::from_millis(10));

        self.peek(&mut peek_buf).await.ok().map_or(false, |ulen| {
            ulen == 4
                && (peek_buf.starts_with("GET ".as_bytes())
                    || peek_buf.starts_with("POST".as_bytes()))
        })
    }
}

impl IsHTTP for UnixStream {
    async fn is_http(&mut self) -> bool {
        return true;
    }
}

pub struct StreamHandler<IO, REPL> {
    inner: IO,
    marker: PhantomData<REPL>,
}

impl<IO, REPL> StreamHandler<IO, REPL>
where
    IO: AsyncRead + AsyncWrite + IsHTTP + std::marker::Unpin,
    REPL: Repl + Default + Send,
{
    pub fn new(inner: IO) -> Self {
        Self {
            inner,
            marker: Default::default(),
        }
    }
    pub async fn run(mut self) -> Result<()> {
        if self.inner.is_http().await {
            self.handle_http().await
        } else {
            self.handle_repl().await
        }
    }

    async fn handle_http(self) -> Result<()> {
        http1::Builder::new()
            .serve_connection(TokioIo::new(self.inner), ProbingService::default())
            .await
            .map_err(|err| err.into())
    }

    async fn handle_repl(mut self) -> Result<()> {
        let mut repl = Box::<REPL>::default();
        let mut buf = [0; 1024];

        let _ = self.inner.write(">>".as_bytes()).await;
        loop {
            let n = match self.inner.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            let req = String::from_utf8(buf[0..n].to_vec());
            let s = match repl.feed(req.clone().unwrap()) {
                Some(rsp) => format!("{}\n>>", rsp),
                None => ">>".to_string(),
            };
            if self.inner.write(s.as_bytes()).await.is_err() {
                break;
            }
        }
        Ok(())
    }
}
