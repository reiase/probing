use serde::{Deserialize, Serialize};
use super::version::ProtocolVersion;

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
        use std::time::{SystemTime, UNIX_EPOCH};
        
        Self {
            version: ProtocolVersion::current(),
            message_id: None,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64,
            payload,
        }
    }
    
    /// Create a new message envelope with a specific message ID
    pub fn with_id(payload: T, id: String) -> Self {
        let mut envelope = Self::new(payload);
        envelope.message_id = Some(id);
        envelope
    }
}
