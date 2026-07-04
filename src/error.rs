//! Error types for TontooData

use thiserror::Error;

/// Main error type for TontooData operations
#[derive(Error, Debug)]
pub enum DataError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Sandbox not initialized - no app ID provided")]
    SandboxNotInitialized,

    #[error("Permission denied - system access required")]
    PermissionDenied,

    #[error("App not found: {0}")]
    AppNotFound(String),

    #[error("Invalid app ID: {0}")]
    InvalidAppId(String),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Encryption/Decryption failed")]
    EncryptionFailed,
}