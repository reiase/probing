use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[allow(unused)]
#[derive(Error, Debug)]
pub enum TCPStoreError {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Timeout error")]
    TimeoutError(#[from] tokio::time::error::Elapsed),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Handler error: {0}")]
    HandlerError(String),
}

#[allow(unused)]
#[repr(u8)]
pub enum QueryType {
    VALIDATE = 0,
    SET = 1,
    COMPARE_SET = 2,
    GET = 3,
    ADD = 4,
    CHECK = 5,
    WAIT = 6,
    GETNUMKEYS = 7,
    DELETE_KEY = 8,
    APPEND = 9,
    MULTI_GET = 10,
    MULTI_SET = 11,
    CANCEL_WAIT = 12,
    PING = 13,
}

pub struct TCPStore {
    endpoint: String,
    keyprefix: String,
    timeout_duration: Option<Duration>,
}

#[allow(unused)]
impl TCPStore {
    /// Create a new TCPStore instance
    pub fn new(endpoint: String) -> Self {
        TCPStore {
            endpoint,
            keyprefix: "/".to_string(),
            timeout_duration: Some(Duration::from_secs(5)), // Default timeout
        }
    }

    /// Set connection timeout
    pub fn with_timeout(mut self, timeout_duration: Duration) -> Self {
        self.timeout_duration = Some(timeout_duration);
        self
    }

    /// Set no timeout
    pub fn with_no_timeout(mut self) -> Self {
        self.timeout_duration = None;
        self
    }

    /// Connect to the target server
    async fn connect(&self) -> Result<TcpStream, TCPStoreError> {
        let connect_future = TcpStream::connect(&self.endpoint);

        let stream = match self.timeout_duration {
            Some(duration) => timeout(duration, connect_future).await?,
            None => connect_future.await,
        }?;
        stream.set_nodelay(true)?;
        Ok(stream)
    }

    /// Send a request and receive a response using the provided handler
    pub async fn run<T, H, F>(&self, handler: H) -> Result<T, TCPStoreError>
    where
        H: FnOnce(TcpStream) -> F,
        F: std::future::Future<Output = Result<T, TCPStoreError>>,
    {
        // Create a new connection (short-lived)
        let stream = self.connect().await?;

        let operation_timeout = Duration::from_secs(10); // Adjust timeout as needed
                                                         // handler(stream).await
        let result = match timeout(operation_timeout, handler(stream)).await {
            Ok(result) => result?,
            Err(elapsed_error) => return Err(TCPStoreError::TimeoutError(elapsed_error)),
        };

        Ok(result)
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<(), TCPStoreError> {
        // Implement the logic to set a key-value pair in the store
        // This is a placeholder implementation

        self.run(|mut stream| async move {
            stream.write_u8(QueryType::VALIDATE as u8).await?;
            stream.write_u32_le(0x3C85F7CE_u32).await?;
            stream.write_u8(QueryType::PING as u8).await?;
            stream.write_u32_le(0_u32).await?;

            let pong = stream.read_u32_le().await?;

            stream.write_u8(QueryType::SET as u8).await?;
            stream
                .write_u64_le((key.len() + self.keyprefix.len()) as u64)
                .await?;
            stream.write_all(self.keyprefix.as_bytes()).await?;
            stream.write_all(key.as_bytes()).await?;
            stream.write_u64_le(value.len() as u64).await?;
            stream.write_all(value.as_bytes()).await?;
            Ok(())
        })
        .await
    }

    pub async fn get(&self, key: &str) -> Result<String, TCPStoreError> {
        self.run(|mut stream| async move {
            stream.write_u8(QueryType::VALIDATE as u8).await?;
            stream.write_u32_le(0x3C85F7CE_u32).await?;
            stream.write_u8(QueryType::PING as u8).await?;
            stream.write_u32_le(0_u32).await?;

            let pong = stream.read_u32_le().await?;

            stream.write_u8(QueryType::GET as u8).await?;
            stream
                .write_u64_le((key.len() + self.keyprefix.len()) as u64)
                .await?;
            stream.write_all(self.keyprefix.as_bytes()).await?;
            stream.write_all(key.as_bytes()).await?;

            // Read the response
            let len = stream.read_u64_le().await?;
            let mut buffer = vec![0; len as usize]; // Adjust size as needed

            let readlen = stream.read_exact(&mut buffer).await?;
            if readlen as u64 != len {
                return Err(TCPStoreError::HandlerError(
                    "Failed to read the expected number of bytes".to_string(),
                ));
            }
            let value = String::from_utf8_lossy(&buffer[..readlen]).to_string();

            Ok(value)
        })
        .await
    }
}
