//! System hooks for Settings app and system management

use crate::{is_system_app, DataError, Result};
use serde::Serialize;
use std::path::PathBuf;

/// Application information for system management
#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    pub id: String,
    pub data_size: u64,
    pub cache_size: u64,
    pub total_size: u64,
}

/// System data access - only available to system apps
pub struct SystemDataAccess {
    apps_root: PathBuf,
    cache_base: PathBuf,
}

impl SystemDataAccess {
    /// Initialize system data access
    /// Requires TONTOO_SYSTEM_APP=true environment variable
    pub fn init() -> Result<Self> {
        if !is_system_app() {
            return Err(DataError::PermissionDenied);
        }

        let apps_root = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tapps");

        let cache_base = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("TontooOS");

        Ok(Self { apps_root, cache_base })
    }

    /// Get storage size for a specific app (SQLite database)
    pub fn get_app_storage_size(&self, app_id: &str) -> Result<u64> {
        let db_path = self.apps_root.join(app_id).join("data.db");
        if !db_path.exists() {
            return Ok(0);
        }
        
        let conn = rusqlite::Connection::open(&db_path)?;
        let size: u64 = conn.query_row(
            "SELECT COALESCE(SUM(size), 0) FROM kv_store",
            [],
            |row| row.get(0),
        )?;
        Ok(size)
    }

    /// Get cache size for a specific app
    pub fn get_app_cache_size(&self, app_id: &str) -> Result<u64> {
        let app_cache = self.cache_base.join(app_id).join("cache");
        let mut total_size: u64 = 0;
        
        if app_cache.exists() {
            for entry in walkdir::WalkDir::new(&app_cache)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter_map(|e| e.metadata().ok())
            {
                if entry.is_file() {
                    total_size += entry.len();
                }
            }
        }
        
        Ok(total_size)
    }

    /// Get complete app info including sizes
    pub fn get_app_info(&self, app_id: &str) -> Result<AppInfo> {
        let data_size = self.get_app_storage_size(app_id)?;
        let cache_size = self.get_app_cache_size(app_id)?;
        
        Ok(AppInfo {
            id: app_id.to_string(),
            data_size,
            cache_size,
            total_size: data_size + cache_size,
        })
    }

    /// List all installed apps with their storage info
    pub fn list_all_apps(&self) -> Result<Vec<AppInfo>> {
        let mut apps = Vec::new();
        
        if !self.apps_root.exists() {
            return Ok(apps);
        }

        for entry in std::fs::read_dir(&self.apps_root)? {
            let entry = entry?;
            let app_id = entry.file_name().to_string_lossy().to_string();
            
            // Skip hidden directories
            if app_id.starts_with('.') {
                continue;
            }

            let info = self.get_app_info(&app_id)?;
            apps.push(info);
        }

        // Sort by total size descending
        apps.sort_by(|a, b| b.total_size.cmp(&a.total_size));
        Ok(apps)
    }

    /// Delete all data for an app (from Settings)
    pub fn delete_app_data(&self, app_id: &str) -> Result<()> {
        let app_db = self.apps_root.join(app_id).join("data.db");
        let app_cache = self.cache_base.join(app_id).join("cache");

        if app_db.exists() {
            std::fs::remove_file(&app_db)?;
        }
        
        if app_cache.exists() {
            std::fs::remove_dir_all(&app_cache)?;
        }

        Ok(())
    }

    /// Clear cache only for an app (user can do this without deleting data)
    pub fn clear_app_cache(&self, app_id: &str) -> Result<()> {
        let app_cache = self.cache_base.join(app_id).join("cache");
        
        if app_cache.exists() {
            std::fs::remove_dir_all(&app_cache)?;
            std::fs::create_dir_all(&app_cache)?;
        }

        Ok(())
    }

    }