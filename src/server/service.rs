use crate::handlers::PPROF_HOLDER;
use crate::repl::{PythonRepl, REPL};
use bytes::Bytes;
use html_render::html;
use http_body_util::Full;
use hyper::service::Service;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use include_dir::include_dir;
use include_dir::Dir;
use pin_project_lite::pin_project;
use pyo3::types::PyAnyMethods;
use pyo3::{Python, ToPyObject};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pin_project! {
    #[derive(Debug)]
    pub struct TokioIo<T> {
        #[pin]
        inner: T,
    }
}

impl<T> TokioIo<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    // pub fn inner(self) -> T {
    //     self.inner
    // }
}

impl<T> hyper::rt::Read for TokioIo<T>
where
    T: tokio::io::AsyncRead,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let n = unsafe {
            let mut tbuf = tokio::io::ReadBuf::uninit(buf.as_mut());
            match tokio::io::AsyncRead::poll_read(self.project().inner, cx, &mut tbuf) {
                Poll::Ready(Ok(())) => tbuf.filled().len(),
                other => return other,
            }
        };

        unsafe {
            buf.advance(n);
        }
        Poll::Ready(Ok(()))
    }
}

impl<T> hyper::rt::Write for TokioIo<T>
where
    T: tokio::io::AsyncWrite,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        tokio::io::AsyncWrite::poll_write(self.project().inner, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        tokio::io::AsyncWrite::poll_flush(self.project().inner, cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        tokio::io::AsyncWrite::poll_shutdown(self.project().inner, cx)
    }

    fn is_write_vectored(&self) -> bool {
        tokio::io::AsyncWrite::is_write_vectored(&self.inner)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        tokio::io::AsyncWrite::poll_write_vectored(self.project().inner, cx, bufs)
    }
}

impl<T> tokio::io::AsyncRead for TokioIo<T>
where
    T: hyper::rt::Read,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        tbuf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        //let init = tbuf.initialized().len();
        let filled = tbuf.filled().len();
        let sub_filled = unsafe {
            let mut buf = hyper::rt::ReadBuf::uninit(tbuf.unfilled_mut());

            match hyper::rt::Read::poll_read(self.project().inner, cx, buf.unfilled()) {
                Poll::Ready(Ok(())) => buf.filled().len(),
                other => return other,
            }
        };

        let n_filled = filled + sub_filled;
        // At least sub_filled bytes had to have been initialized.
        let n_init = sub_filled;
        unsafe {
            tbuf.assume_init(n_init);
            tbuf.set_filled(n_filled);
        }

        Poll::Ready(Ok(()))
    }
}

impl<T> tokio::io::AsyncWrite for TokioIo<T>
where
    T: hyper::rt::Write,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        hyper::rt::Write::poll_write(self.project().inner, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        hyper::rt::Write::poll_flush(self.project().inner, cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        hyper::rt::Write::poll_shutdown(self.project().inner, cx)
    }

    fn is_write_vectored(&self) -> bool {
        hyper::rt::Write::is_write_vectored(&self.inner)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[std::io::IoSlice<'_>],
    ) -> Poll<Result<usize, std::io::Error>> {
        hyper::rt::Write::poll_write_vectored(self.project().inner, cx, bufs)
    }
}

type Counter = i32;
const DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/dist");
#[derive(Default, Debug, Clone)]
pub(crate) struct Svc {
    counter: Arc<Mutex<Counter>>,
}

impl Service<Request<IncomingBody>> for Svc {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        let path = req.uri().path().to_string();
        let mk_raw_response = &move |s: Bytes| -> Result<Response<Full<Bytes>>, hyper::Error> {
            let builder = Response::builder();
            let builder = if path.ends_with(".html") {
                builder.header("Content-Type", "text/html")
            } else if path.ends_with(".js") {
                builder.header("Content-Type", "application/javascript")
            } else if path.ends_with(".css") {
                builder.header("Content-Type", "text/css")
            } else if path.ends_with(".wasm") {
                builder.header("Content-Type", "application/wasm")
            } else {
                builder
            };
            Ok(builder.body(Full::new(s)).unwrap())
        };
        let mk_response = move |s: String| -> Result<Response<Full<Bytes>>, hyper::Error> {
            mk_raw_response(Bytes::from(s))
        };

        if req.uri().path() != "/favicon.ico" {
            *self.counter.lock().expect("lock poisoned") += 1;
        }

        let path = req.uri().path();
        let path = if path == "/" { "/index.html" } else { path };
        let res = match path {
            "/apis" => mk_response(
                html! {
                    <div>
                    <body>
                    <p><a href="/flamegraph">{"flamegraph"}</a></p>
                    <p><a href="/objects">{"objects"}</a></p>
                    <p><a href="/torch/tensors">{"torch.Tensor"}</a></p>
                    <p><a href="/torch/modules">{"torch.nn.Module"}</a></p>
                    </body>
                    </div>
                }
                .to_string(),
            ),
            "/backtrace" => {
                let ret = Python::with_gil(|py| {
                    let ret = py
                        .import_bound("traceback")
                        .unwrap()
                        .call_method0("format_stack")
                        .unwrap_or_else(|err| {
                            err.print(py);
                            err.to_string().to_object(py).into_bound(py)
                        });
                    let ret = "\n"
                        .to_object(py)
                        .call_method1(py, "join", (ret.as_unbound(),));
                    match ret {
                        Ok(obj) => obj.to_string(),
                        Err(err) => {
                            err.print(py);
                            err.to_string()
                        }
                    }
                });
                mk_response(ret)
            }
            "/flamegraph" => {
                let report = PPROF_HOLDER
                    .flamegraph()
                    .unwrap_or("no profile data".to_string());
                mk_response(report)
            }
            "/flamegraph.svg" => {
                let report = PPROF_HOLDER
                    .flamegraph()
                    .unwrap_or("no profile data".to_string());
                mk_response(report)
            }
            path if DIST.contains(path.trim_start_matches('/')) => {
                let file = DIST.get_file(path.trim_start_matches('/')).unwrap();
                let content = Bytes::copy_from_slice(file.contents());
                mk_raw_response(content)
            }
            path => {
                let request = format!(
                    "handle(path=\"{}\", query={})\n",
                    path,
                    req.uri()
                        .query()
                        .map(|qs| { format!("\"{}\"", qs) })
                        .unwrap_or("None".to_string())
                );
                let mut repl = PythonRepl::default();
                let ret = repl.process(request.as_str());
                mk_response(ret.unwrap_or("".to_string()))
            }
        };

        Box::pin(async { res })
    }
}
