//! App sandboxing implementation

use crate::{app_id, DataError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use walkdir::WalkDir;

/// App sandbox for isolated data storage
pub struct TontooData {
    app_id: String,
    data_root: PathBuf,
    cache_root: PathBuf,
}

impl TontooData {
    /// Initialize data storage for the current app
    /// 
    /// Uses TONTOO_APP_ID environment variable or provided app_id
    pub fn init() -> Result<Self> {
        Self::with_id(app_id().ok_or(DataError::SandboxNotInitialized)?)
    }

    /// Initialize with explicit app ID
    pub fn with_id(app_id: impl Into<String>) -> Result<Self> {
        let app_id = app_id.into();
        
        if app_id.is_empty() {
            return Err(DataError::InvalidAppId("App ID cannot be empty".to_string()));
        }

        let data_root = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("TontooOS")
            .join("Apps")
            .join(&app_id)
            .join("data");

        let cache_root = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("TontooOS")
            .join(&app_id)
            .join("cache");

        // Create directories with restricted permissions (700)
        std::fs::create_dir_all(&data_root)?;
        std::fs::create_dir_all(&cache_root)?;

        // Set directory permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(&data_root, perms)?;
            std::fs::set_permissions(&cache_root, perms)?;
        }

        Ok(Self { app_id, data_root, cache_root })
    }

    /// Save data to the sandbox
    pub fn save<T: Serialize>(&self, key: &str, data: &T) -> Result<()> {
        let path = self.data_root.join(format!("{}.json", key));
        let json = serde_json::to_string_pretty(data)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Load data from the sandbox
    pub fn load<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let path = self.data_root.join(format!("{}.json", key));
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        let data = serde_json::from_str(&content)?;
        Ok(Some(data))
    }

    /// Delete data from the sandbox
    pub fn delete(&self, key: &str) -> Result<()> {
        let path = self.data_root.join(format!("{}.json", key));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Get path for file storage (for large files)
    pub fn file_path(&self, filename: &str) -> PathBuf {
        self.data_root.join(filename)
    }

    /// Save binary data to a file
    pub fn save_file(&self, filename: &str, data: &[u8]) -> Result<()> {
        let path = self.file_path(filename);
        std::fs::write(&path, data)?;
        Ok(())
    }

    /// Load binary data from a file
    pub fn load_file(&self, filename: &str) -> Result<Option<Vec<u8>>> {
        let path = self.file_path(filename);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read(&path)?;
        Ok(Some(content))
    }

    /// Clear all data for this app (for cleanup hooks)
    pub fn clear_all_data(&self) -> Result<()> {
        if self.data_root.exists() {
            std::fs::remove_dir_all(&self.data_root)?;
            std::fs::create_dir_all(&self.data_root)?;
        }
        Ok(())
    }

    /// Clear cache for this app
    pub fn clear_cache(&self) -> Result<()> {
        if self.cache_root.exists() {
            std::fs::remove_dir_all(&self.cache_root)?;
            std::fs::create_dir_all(&self.cache_root)?;
        }
        Ok(())
    }

    /// Get total data size in bytes
    pub fn get_data_size(&self) -> Result<u64> {
        self.calculate_dir_size(&self.data_root)
    }

    /// Get total cache size in bytes
    pub fn get_cache_size(&self) -> Result<u64> {
        self.calculate_dir_size(&self.cache_root)
    }

    /// Get app ID
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    fn calculate_dir_size(&self, path: &PathBuf) -> Result<u64> {
        let size = WalkDir::new(path)
            .into_iter()
            .map(|entry| entry.map(|e| e.metadata().map(|m| m.len() if m.is_file() else 0).unwrap_or(0)))
            .sum::<u64>();
        Ok(size)
    }
}