//! TontooData - Data storage library for TontooOS
//!
//! Provides macOS-style sandboxed data storage with system hooks for management.

mod error;
mod sandbox;
mod system_hooks;
mod query;

pub use error::DataError;
pub use sandbox::TontooData;
pub use system_hooks::SystemDataAccess;
pub use query::{Batch, BatchOp, Model, Migration, Predicate, Relationship, RelationshipKind, SchemaManager, SortDescriptor, Query};

/// Check if running as system app (for Settings app)
pub fn is_system_app() -> bool {
    std::env::var("TONTOO_SYSTEM_APP").unwrap_or_default() == "true"
}

/// Read the app id from `Tontoo.proj`.
/// We walk up from the current executable to find `Tontoo.proj`,
/// then parse the `<Id>` tag from the XML.
pub fn get_app_id() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?;

    loop {
        let candidate = dir.join("Tontoo.proj");
        if candidate.exists() {
            let content = std::fs::read_to_string(candidate).ok()?;
            return parse_app_id_from_proj(&content);
        }

        if dir.parent() == Some(dir.clone()) {
            break;
        }
        dir = dir.parent()?;
    }

    None
}

fn parse_app_id_from_proj(xml: &str) -> Option<String> {
    let doc = quick_xml::de::from_str::<TontooProj>(xml).ok()?;
    Some(doc.app_info.id)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename = "TontooProject")]
struct TontooProj {
    #[serde(rename = "AppInfo")]
    app_info: AppInfoXml,
}

#[derive(Debug, serde::Deserialize)]
struct AppInfoXml {
    #[serde(rename = "Id")]
    id: String,
}

/// Re-export common Result type
pub type Result<T> = std::result::Result<T, DataError>;