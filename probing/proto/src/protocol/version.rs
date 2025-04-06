use serde::{Deserialize, Serialize};

/// Protocol version information
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProtocolVersion {
    /// Major version - incremented for breaking changes
    pub major: u16,
    /// Minor version - incremented for backwards-compatible feature additions
    pub minor: u16,
    /// Patch version - incremented for backwards-compatible bug fixes
    pub patch: u16,
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self {
            major: 0,
            minor: 1,
            patch: 0,
        }
    }
}

impl ProtocolVersion {
    /// Check if this version is compatible with another version
    pub fn is_compatible_with(&self, other: &ProtocolVersion) -> bool {
        self.major == other.major
    }
    
    /// Get the current protocol version
    pub fn current() -> Self {
        Self::default()
    }
}
