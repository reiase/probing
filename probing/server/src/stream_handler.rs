use std::sync::Arc;

use anyhow::Result;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Request;
use hyper::Response;
use hyper::body::Bytes;
use probing_core::Probe;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;

use super::tokio_io::TokioIo;
// use crate::repl::Repl;
// use crate::service::handle_request;

pub async fn handle_request(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>> {
    Ok(Default::default())
}

pub struct StreamHandler<IO> {
    inner: IO,
    probe: Arc<dyn Probe>,
}

impl<IO> StreamHandler<IO>
where
    IO: AsyncRead + AsyncWrite + std::marker::Unpin,
{
    pub fn new(inner: IO, probe: Arc<dyn Probe>) -> Self {
        Self { inner, probe }
    }

    pub async fn run(self) -> Result<()> {
        self.handle_http().await
    }

    async fn handle_http(self) -> Result<()> {
        http1::Builder::new()
            // .serve_connection(TokioIo::new(self.inner), ProbingService::default())
            .serve_connection(TokioIo::new(self.inner), service_fn(handle_request))
            .await
            .map_err(|err| err.into())
    }
}
