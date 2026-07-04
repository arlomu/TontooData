//! App sandboxing implementation with SQLite backend

use crate::{get_app_id, DataError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// App sandbox for isolated data storage with SQLite backend
pub struct TontooData {
    app_id: String,
    db_path: PathBuf,
    cache_root: PathBuf,
    conn: Option<rusqlite::Connection>,
}

impl TontooData {
    /// Initialize data storage for the current app
    /// 
    /// Uses TONTOO_APP_ID environment variable
    /// Note: Encryption is controlled via `encryption` Cargo feature
    pub fn init() -> Result<Self> {
        Self::with_id(get_app_id().ok_or(DataError::SandboxNotInitialized)?)
    }

    /// Initialize with explicit app ID
    /// 
    /// Note: Encryption is controlled via Cargo feature flag
    pub fn with_id(app_id: impl Into<String>) -> Result<Self> {
        let app_id = app_id.into();
        
        if app_id.is_empty() {
            return Err(DataError::InvalidAppId("App ID cannot be empty".to_string()));
        }

        let db_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tapps")
            .join(&app_id)
            .join("data.db");

        let cache_root = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("TontooOS")
            .join(&app_id)
            .join("cache");

        // Create directories with restricted permissions (700)
        std::fs::create_dir_all(db_path.parent().unwrap())?;
        std::fs::create_dir_all(&cache_root)?;

        // Set directory permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(db_path.parent().unwrap(), perms)?;
            std::fs::set_permissions(&cache_root, perms)?;
        }

        let mut instance = Self { 
            app_id, 
            db_path, 
            cache_root, 
            conn: None 
        };
        
        // Initialize database connection
        instance.init_database()?;
        
        Ok(instance)
    }

    fn init_database(&mut self) -> Result<()> {
        let conn = rusqlite::Connection::open(&self.db_path)?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS kv_store (
                key TEXT PRIMARY KEY,
                value BLOB,
                size INTEGER DEFAULT 0
            )",
            [],
        )?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT
            )",
            [],
        )?;
        
        self.conn = Some(conn);
        Ok(())
    }

    /// Save data to the sandbox (lazy - uses SQLite)
    pub fn save<T: Serialize>(&mut self, key: &str, data: &T) -> Result<()> {
        #[cfg(feature = "encryption")]
        let serialized = self.encrypt_data(serde_json::to_vec(data)?)?;
        
        #[cfg(not(feature = "encryption"))]
        let serialized = serde_json::to_vec(data)?;
        
        let conn = self.conn.as_ref().ok_or_else(|| DataError::SandboxNotInitialized)?;
        
        conn.execute(
            "INSERT OR REPLACE INTO kv_store (key, value, size) VALUES (?1, ?2, ?3)",
            rusqlite::params![key, serialized, serialized.len() as u64],
        )?;
        
        Ok(())
    }

    /// Load data from the sandbox (lazy loading)
    pub fn load<T: for<'de> Deserialize<'de>>(&mut self, key: &str) -> Result<Option<T>> {
        let conn = self.conn.as_ref().ok_or_else(|| DataError::SandboxNotInitialized)?;
        
        let mut stmt = conn.prepare("SELECT value FROM kv_store WHERE key = ?1")?;
        
        let result = stmt.query_map([key], |row| {
            let value: Vec<u8> = row.get(0)?;
            Ok(value)
        })?
        .next()
        .transpose()?;
        
        match result {
            Some(bytes) => {
                #[cfg(feature = "encryption")]
                let deserialized = self.decrypt_data(&bytes)?;
                
                #[cfg(not(feature = "encryption"))]
                let deserialized = bytes;
                
                let data: T = serde_json::from_slice(&deserialized)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Delete data from the sandbox
    pub fn delete(&mut self, key: &str) -> Result<()> {
        let conn = self.conn.as_ref().ok_or_else(|| DataError::SandboxNotInitialized)?;
        
        conn.execute("DELETE FROM kv_store WHERE key = ?1", [key])?;
        Ok(())
    }

    /// Get path for file storage (for large files)
    pub fn file_path(&self, filename: &str) -> PathBuf {
        self.db_path.parent()
            .unwrap_or(&PathBuf::from("."))
            .join(filename)
    }

    /// Save binary data to a file
    pub fn save_file(&mut self, filename: &str, data: &[u8]) -> Result<()> {
        let path = self.file_path(filename);
        std::fs::write(&path, data)?;
        
        // Track in database
        let conn = self.conn.as_ref().ok_or_else(|| DataError::SandboxNotInitialized)?;
        conn.execute(
            "INSERT OR REPLACE INTO kv_store (key, value, size) VALUES (?1, ?2, ?3)",
            rusqlite::params![format!("file:{}", filename), data, data.len() as u64],
        )?;
        
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
    pub fn clear_all_data(&mut self) -> Result<()> {
        if let Some(conn) = self.conn.take() {
            let _ = conn.close();
        }
        
        if self.db_path.exists() {
            std::fs::remove_file(&self.db_path)?;
        }
        
        self.conn = None;
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
    pub fn get_data_size(&mut self) -> Result<u64> {
        let conn = self.conn.as_ref().ok_or_else(|| DataError::SandboxNotInitialized)?;
        
        let size: u64 = conn.query_row(
            "SELECT COALESCE(SUM(size), 0) FROM kv_store",
            [],
            |row| row.get(0),
        )?;
        
        Ok(size)
    }

    /// Get app ID
    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    /// Get all keys in the database
    pub fn list_keys(&mut self) -> Result<Vec<String>> {
        let conn = self.conn.as_ref().ok_or_else(|| DataError::SandboxNotInitialized)?;
        
        let mut stmt = conn.prepare("SELECT key FROM kv_store")?;
        let keys = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|e| e.ok())
            .collect();
        
        Ok(keys)
    }

    /// Execute batch operations
    pub fn batch<T: Serialize + for<'de> Deserialize<'de>>(&mut self, ops: &[(&str, Option<T>)]) -> Result<()> {
        let conn = self.conn.as_ref().ok_or_else(|| DataError::SandboxNotInitialized)?;
        
        for (key, data) in ops {
            match data {
                Some(value) => {
                    let serialized = serde_json::to_vec(value)?;
                    conn.execute(
                        "INSERT OR REPLACE INTO kv_store (key, value, size) VALUES (?1, ?2, ?3)",
                        rusqlite::params![key, serialized, serialized.len() as u64],
                    )?;
                }
                None => {
                    conn.execute("DELETE FROM kv_store WHERE key = ?1", [key])?;
                }
            }
        }
        Ok(())
    }

    /// Flush database changes to disk
    pub fn flush(&mut self) -> Result<()> {
        if let Some(conn) = &self.conn {
            conn.execute("PRAGMA wal_checkpoint(FULL)", [])?;
        }
        Ok(())
    }
}

#[cfg(feature = "encryption")]
impl TontooData {
    fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        
        let key_bytes = self.app_id.as_bytes();
        let hash = blake3::hash(key_bytes);
        let key = Key::from_slice(&hash);
        let nonce = Nonce::from_slice(b"TntOS_Encrypt!");
        
        let cipher = Aes256Gcm::new(key);
        cipher.encrypt(nonce, data)
            .map_err(|_| DataError::EncryptionFailed)
    }

    fn decrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Key, Nonce};
        
        let key_bytes = self.app_id.as_bytes();
        let hash = blake3::hash(key_bytes);
        let key = Key::from_slice(&hash);
        let nonce = Nonce::from_slice(b"TntOS_Encrypt!");
        
        let cipher = Aes256Gcm::new(key);
        cipher.decrypt(nonce, data)
            .map_err(|_| DataError::EncryptionFailed)
    }
}