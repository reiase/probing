/// Maximum request body size allowed (5MB)
pub const MAX_REQUEST_BODY_SIZE: usize = 5 * 1024 * 1024;

/// Maximum file size allowed for file API reading (10MB)
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Allowed base directories for file access
pub const ALLOWED_FILE_DIRS: &[&str] = &[
    "./logs", "./data", "./config",
    // Add more allowed directories as needed
];

/// Get maximum request body size from environment or use default
pub fn get_max_request_body_size() -> usize {
    std::env::var("PROBING_MAX_REQUEST_SIZE")
        .unwrap_or(MAX_REQUEST_BODY_SIZE.to_string())
        .parse::<usize>()
        .unwrap_or(MAX_REQUEST_BODY_SIZE)
}

/// Get maximum file size from environment or use default
pub fn get_max_file_size() -> u64 {
    std::env::var("PROBING_MAX_FILE_SIZE")
        .unwrap_or(MAX_FILE_SIZE.to_string())
        .parse::<u64>()
        .unwrap_or(MAX_FILE_SIZE)
}
