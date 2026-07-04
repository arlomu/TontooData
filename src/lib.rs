//! TontooData - Data storage library for TontooOS
//!
//! Provides macOS-style sandboxed data storage with system hooks for management.

mod error;
mod sandbox;
mod system_hooks;

pub use error::DataError;
pub use sandbox::TontooData;
pub use system_hooks::SystemDataAccess;

/// Check if running as system app (for Settings app)
pub fn is_system_app() -> bool {
    std::env::var("TONTOO_SYSTEM_APP").unwrap_or_default() == "true"
}

/// Get the application identifier from environment
pub fn app_id() -> Option<String> {
    std::env::var("TONTOO_APP_ID").ok()
}

/// Re-export common Result type
pub type Result<T> = std::result::Result<T, DataError>;

/// Get the application identifier from environment
pub fn app_id() -> Option<String> {
    std::env::var("TONTOO_APP_ID").ok()
}