use super::version::ProtocolVersion;
use serde::{Deserialize, Serialize};

/// A common envelope for all protocol messages
///
/// Provides protocol metadata such as versioning and message type
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Message<T> {
    /// Protocol version
    pub version: ProtocolVersion,

    /// Message ID for correlating requests and responses
    pub message_id: Option<String>,

    /// Timestamp (in microseconds since epoch)
    pub timestamp: u64,

    /// The actual message payload
    pub payload: T,
}

impl<T> Message<T> {
    /// Create a new message envelope with the current protocol version
    pub fn new(payload: T) -> Self {
        Self {
            version: ProtocolVersion::current(),
            message_id: None,
            timestamp: Self::now(),
            payload,
        }
    }

    /// Create a new message envelope with a specific message ID
    pub fn with_id(payload: T, id: String) -> Self {
        let mut envelope = Self::new(payload);
        envelope.message_id = Some(id);
        envelope
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn now() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }

    #[cfg(target_arch = "wasm32")]
    fn now() -> u64 {
        // 使用web-sys获取高精度时间戳
        // 如果不可用则返回Date.now()的毫秒时间戳转换为微秒
        #[cfg(feature = "web")]
        {
            web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| (p.now() * 1000.0) as u64)
                .unwrap_or_else(|| js_sys::Date::now() as u64 * 1000)
        }
        #[cfg(not(feature = "web"))]
        {
            // 对于非web环境的wasm32，返回固定值或使用其他方案
            0
        }
    }
}
