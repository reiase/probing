use crate::handlers::PPROF_HOLDER;
use crate::repl::{PythonRepl, REPL};
use bytes::Bytes;
use html_render::html;
use http_body_util::Full;
use hyper::service::Service;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use pin_project_lite::pin_project;
use pyo3::types::PyAnyMethods;
use pyo3::{Python, ToPyObject};
use qstring::QString;
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

#[derive(Debug, Clone)]
pub(crate) struct Svc {
    counter: Arc<Mutex<Counter>>,
}

impl Default for Svc {
    fn default() -> Self {
        Self {
            counter: Default::default(),
        }
    }
}

impl Service<Request<IncomingBody>> for Svc {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<IncomingBody>) -> Self::Future {
        fn mk_response(s: String) -> Result<Response<Full<Bytes>>, hyper::Error> {
            Ok(Response::builder().body(Full::new(Bytes::from(s))).unwrap())
        }

        if req.uri().path() != "/favicon.ico" {
            *self.counter.lock().expect("lock poisoned") += 1;
        }

        let res = match req.uri().path() {
            "/" => mk_response(
                html! {
                    <div>
                    <body>
                    <p><a href="/flamegraph">{"flamegraph"}</a></p>
                    <p><a href="/objects">{"objects"}</a></p>
                    </body>
                    </div>
                }
                .to_string(),
            ),
            "/flamegraph" => {
                mk_response(PPROF_HOLDER.flamegraph().unwrap_or("default".to_string()))
            }
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
            "/torch" => {mk_response("not implemented".to_string())}
            "/torch/tensors" => {mk_response("not implemented".to_string())}
            "/torch/modules" => {mk_response("not implemented".to_string())}
            s => {
                if s.starts_with("/objects") {
                    let mut filters: Vec<String> = vec![];
                    if let Some(q) = req.uri().query() {
                        let params = QString::from(format!("?{}", q).as_str());
                        params.get("type").map(|val| {
                            filters.push(format!("type_selector=\"{}\"", val));
                        });
                        params.get("limit").map(|val| {
                            filters.push(format!("limit={}", val));
                        });
                    }
                    let query = if filters.is_empty() {
                        "objects()\n".to_string()
                    } else {
                        format!("objects({})\n", filters.join(", "))
                    };
                    let mut repl = PythonRepl::default();
                    let ret = repl.feed(query);
                    mk_response(ret.unwrap_or("[]".to_string()))
                } else {
                    mk_response("oh no! not found".into())
                }
            }
        };

        Box::pin(async { res })
    }
}
