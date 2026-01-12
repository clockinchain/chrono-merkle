//! Persistent storage backends for ChronoMerkle Tree

#[cfg(feature = "storage")]
use crate::tree::StorageBackend;

#[cfg(feature = "no-std")]
use alloc::{boxed::Box, collections::BTreeMap, format, string::String, vec::Vec};
#[cfg(not(feature = "no-std"))]
use std::{collections::BTreeMap, string::String, vec::Vec};

/// In-memory storage backend for testing and development
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct MemoryStorage {
    data: BTreeMap<String, Vec<u8>>,
}

impl MemoryStorage {
    /// Create a new in-memory storage backend
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(feature = "storage")]
impl StorageBackend for MemoryStorage {
    fn save(&mut self, key: &str, data: &[u8]) -> Result<()> {
        self.data.insert(String::from(key), data.to_vec());
        Ok(())
    }

    fn load(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.data.get(key).cloned())
    }

    fn delete(&mut self, key: &str) -> Result<()> {
        self.data.remove(key);
        Ok(())
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        Ok(self.data.keys().cloned().collect())
    }

    fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.data.contains_key(key))
    }
}

/// File-based storage backend
#[cfg(all(feature = "storage", feature = "std", not(feature = "no-std")))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct FileStorage {
    base_path: std::path::PathBuf,
}

#[cfg(all(feature = "storage", feature = "std", not(feature = "no-std")))]
impl FileStorage {
    /// Create a new file-based storage backend
    pub fn new<P: Into<std::path::PathBuf>>(path: P) -> Self {
        Self {
            base_path: path.into(),
        }
    }

    /// Ensure the base directory exists
    fn ensure_base_dir(&self) -> Result<()> {
        let dir_path = if self.base_path.is_dir() {
            &self.base_path
        } else {
            self.base_path.parent().unwrap_or(&self.base_path)
        };

        std::fs::create_dir_all(dir_path).map_err(|e| {
            crate::ChronoMerkleError::StorageError {
                reason: format!("Failed to create directory: {}", e),
            }
        })?;
        Ok(())
    }

    /// Get the full path for a key
    fn key_path(&self, key: &str) -> std::path::PathBuf {
        self.base_path.join(format!("{}.dat", key))
    }
}

#[cfg(all(feature = "storage", feature = "std", not(feature = "no-std")))]
impl StorageBackend for FileStorage {
    fn save(&mut self, key: &str, data: &[u8]) -> Result<()> {
        self.ensure_base_dir()?;
        let path = self.key_path(key);
        std::fs::write(&path, data).map_err(|e| {
            crate::error::ChronoMerkleError::StorageError {
                reason: format!("Failed to write file {}: {}", path.display(), e),
            }
        })
    }

    fn load(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let path = self.key_path(key);
        match std::fs::read(&path) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ChronoMerkleError::StorageError {
                reason: format!("Failed to read file {}: {}", path.display(), e),
            }),
        }
    }

    fn delete(&mut self, key: &str) -> Result<()> {
        let path = self.key_path(key);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(ChronoMerkleError::StorageError {
                reason: format!("Failed to delete file {}: {}", path.display(), e),
            }),
        }
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        let dir_path = if self.base_path.is_dir() {
            &self.base_path
        } else {
            self.base_path.parent().unwrap_or(&self.base_path)
        };

        if !dir_path.exists() {
            return Ok(Vec::new());
        }

        let entries = std::fs::read_dir(dir_path).map_err(|e| {
            crate::ChronoMerkleError::StorageError {
                reason: format!("Failed to read directory: {}", e),
            }
        })?;

        let mut keys = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| {
                crate::ChronoMerkleError::StorageError {
                    reason: format!("Failed to read directory entry: {}", e),
                }
            })?;

            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".dat") {
                    if let Some(key) = file_name.strip_suffix(".dat") {
                        keys.push(String::from(key));
                    }
                }
            }
        }

        Ok(keys)
    }

    fn exists(&self, key: &str) -> Result<bool> {
        let path = self.key_path(key);
        Ok(path.exists())
    }
}

/// Extended storage traits for advanced features
pub mod extensions {
    use super::*;

    /// Trait for storage backends that support batch operations
    #[cfg(feature = "storage")]
    pub trait BatchStorageBackend: StorageBackend {
        /// Save multiple key-value pairs in a single operation
        fn save_batch(&mut self, items: Vec<(&str, &[u8])>) -> Result<()> {
            for (key, data) in items {
                self.save(key, data)?;
            }
            Ok(())
        }

        /// Load multiple keys in a single operation
        fn load_batch(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>> {
            keys.iter().map(|key| self.load(key)).collect()
        }

        /// Delete multiple keys in a single operation
        fn delete_batch(&mut self, keys: &[&str]) -> Result<()> {
            for key in keys {
                self.delete(key)?;
            }
            Ok(())
        }
    }

    /// Trait for storage backends that support atomic operations
    #[cfg(feature = "storage")]
    pub trait AtomicStorageBackend: StorageBackend {
        /// Execute a transaction with multiple operations
        fn transaction<F, R>(&mut self, f: F) -> Result<R>
        where
            F: FnOnce(&mut Self) -> Result<R>;
    }

    /// Configuration for storage backends
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug, Clone)]
    pub struct StorageConfig {
        /// Maximum number of concurrent connections
        pub max_connections: Option<usize>,
        /// Connection timeout in seconds
        pub timeout_seconds: Option<u64>,
        /// Enable compression
        pub compression_enabled: bool,
        /// Enable encryption
        pub encryption_enabled: bool,
        /// Encryption key (if encryption is enabled)
        pub encryption_key: Option<Vec<u8>>,
        /// Custom configuration options
        pub custom_options: BTreeMap<String, String>,
    }

    impl Default for StorageConfig {
        fn default() -> Self {
            Self {
                max_connections: None,
                timeout_seconds: None,
                compression_enabled: false,
                encryption_enabled: false,
                encryption_key: None,
                custom_options: BTreeMap::new(),
            }
        }
    }
}

/// Compressed storage wrapper that automatically compresses/decompresses data
#[cfg(all(feature = "storage", feature = "compressed-storage", feature = "std", not(feature = "no-std")))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct CompressedStorage<S: StorageBackend> {
    inner: S,
    compression_level: i32, // 0-9, where 9 is maximum compression
}

#[cfg(all(feature = "storage", feature = "compressed-storage", feature = "std", not(feature = "no-std")))]
impl<S: StorageBackend> CompressedStorage<S> {
    /// Create a new compressed storage wrapper
    pub fn new(inner: S, compression_level: i32) -> Self {
        Self {
            inner,
            compression_level: compression_level.clamp(0, 9),
        }
    }

    /// Compress data using deflate algorithm
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use std::io::Write;
        let mut encoder = flate2::write::DeflateEncoder::new(
            Vec::new(),
            flate2::Compression::new(self.compression_level as u32),
        );
        Write::write_all(&mut encoder, data).map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Compression failed: {}", e),
            }
        })?;
        encoder.finish().map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Compression finish failed: {}", e),
            }
        })
    }

    /// Decompress data using deflate algorithm
    fn decompress_data(&self, compressed_data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = flate2::read::DeflateDecoder::new(compressed_data);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Decompression failed: {}", e),
            }
        })?;
        Ok(decompressed)
    }
}

#[cfg(all(feature = "storage", feature = "compressed-storage", feature = "std", not(feature = "no-std")))]
impl<S: StorageBackend> StorageBackend for CompressedStorage<S> {
    fn save(&mut self, key: &str, data: &[u8]) -> Result<()> {
        let compressed = self.compress_data(data)?;
        self.inner.save(key, &compressed)
    }

    fn load(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self.inner.load(key)? {
            Some(compressed) => {
                let decompressed = self.decompress_data(&compressed)?;
                Ok(Some(decompressed))
            }
            None => Ok(None),
        }
    }

    fn delete(&mut self, key: &str) -> Result<()> {
        self.inner.delete(key)
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        self.inner.list_keys()
    }

    fn exists(&self, key: &str) -> Result<bool> {
        self.inner.exists(key)
    }
}

/// Encrypted storage wrapper that automatically encrypts/decrypts data
#[cfg(all(feature = "storage", feature = "encrypted-storage"))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct EncryptedStorage<S: StorageBackend> {
    inner: S,
    key: Vec<u8>,
}

#[cfg(all(feature = "storage", feature = "encrypted-storage"))]
impl<S: StorageBackend> EncryptedStorage<S> {
    /// Create a new encrypted storage wrapper with AES-256-GCM
    pub fn new(inner: S, key: Vec<u8>) -> Result<Self> {
        if key.len() != 32 {
            return Err(ChronoMerkleError::StorageError {
                reason: String::from("Encryption key must be 32 bytes for AES-256"),
            });
        }
        Ok(Self { inner, key })
    }

    /// Encrypt data using AES-256-GCM
    fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Nonce, KeyInit};
        use aes_gcm::aead::Aead;
        use rand::RngCore;

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(&key);

        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, data).map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Encryption failed: {}", e),
            }
        })?;

        // Prepend nonce to ciphertext for decryption
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    /// Decrypt data using AES-256-GCM
    fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, Nonce, KeyInit};
        use aes_gcm::aead::Aead;

        if encrypted_data.len() < 12 {
            return Err(ChronoMerkleError::StorageError {
                reason: String::from("Encrypted data too short"),
            });
        }

        let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&self.key);
        let cipher = Aes256Gcm::new(&key);

        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];

        cipher.decrypt(nonce, ciphertext).map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Decryption failed: {}", e),
            }
        })
    }
}

#[cfg(all(feature = "storage", feature = "encrypted-storage"))]
impl<S: StorageBackend> StorageBackend for EncryptedStorage<S> {
    fn save(&mut self, key: &str, data: &[u8]) -> Result<()> {
        let encrypted = self.encrypt_data(data)?;
        self.inner.save(key, &encrypted)
    }

    fn load(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self.inner.load(key)? {
            Some(encrypted) => {
                let decrypted = self.decrypt_data(&encrypted)?;
                Ok(Some(decrypted))
            }
            None => Ok(None),
        }
    }

    fn delete(&mut self, key: &str) -> Result<()> {
        self.inner.delete(key)
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        self.inner.list_keys()
    }

    fn exists(&self, key: &str) -> Result<bool> {
        self.inner.exists(key)
    }
}

/// PostgreSQL-based storage backend
#[cfg(all(feature = "storage", feature = "postgres-storage", not(feature = "no-std")))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct PostgresStorage {
    connection_string: String,
    #[cfg(feature = "serde")]
    #[serde(skip)]
    pool: Option<tokio_postgres::Client>,
}

#[cfg(all(feature = "storage", feature = "postgres-storage", not(feature = "no-std")))]
impl PostgresStorage {
    /// Create a new PostgreSQL storage backend
    pub async fn new(connection_string: String) -> Result<Self> {
        let (client, connection) = tokio_postgres::connect(&connection_string, tokio_postgres::NoTls)
            .await
            .map_err(|e| ChronoMerkleError::StorageError {
                reason: format!("Failed to connect to PostgreSQL: {}", e),
            })?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                std::eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        // Create table if it doesn't exist
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS chrono_merkle_storage (
                    key TEXT PRIMARY KEY,
                    data BYTEA NOT NULL,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
                )",
                &[],
            )
            .await
            .map_err(|e| ChronoMerkleError::StorageError {
                reason: format!("Failed to create table: {}", e),
            })?;

        Ok(Self {
            connection_string,
            pool: Some(client),
        })
    }

    /// Get a reference to the database client
    fn client(&self) -> Result<&tokio_postgres::Client> {
        self.pool.as_ref().ok_or_else(|| ChronoMerkleError::StorageError {
            reason: String::from("Database connection not available"),
        })
    }
}

#[cfg(all(feature = "storage", feature = "postgres-storage", not(feature = "no-std")))]
impl StorageBackend for PostgresStorage {
    fn save(&mut self, key: &str, data: &[u8]) -> Result<()> {
        let client = self.client()?;
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Failed to create runtime: {}", e),
            }
        })?;

        rt.block_on(async {
            let _ = client
                .execute(
                    "INSERT INTO chrono_merkle_storage (key, data, updated_at) VALUES ($1, $2, NOW())
                     ON CONFLICT (key) DO UPDATE SET data = EXCLUDED.data, updated_at = NOW()",
                    &[&key, &&data[..]],
                )
                .await
                .map_err(|e| ChronoMerkleError::StorageError {
                    reason: format!("Failed to save data: {}", e),
                })?;
            Ok(())
        })
    }

    fn load(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let client = self.client()?;
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Failed to create runtime: {}", e),
            }
        })?;

        rt.block_on(async {
            let rows = client
                .query("SELECT data FROM chrono_merkle_storage WHERE key = $1", &[&key])
                .await
                .map_err(|e| ChronoMerkleError::StorageError {
                    reason: format!("Failed to load data: {}", e),
                })?;

            if let Some(row) = rows.first() {
                let data: &[u8] = row.get::<_, &[u8]>(0);
                Ok(Some(data.to_vec()))
            } else {
                Ok(None)
            }
        })
    }

    fn delete(&mut self, key: &str) -> Result<()> {
        let client = self.client()?;
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Failed to create runtime: {}", e),
            }
        })?;

        rt.block_on(async {
            let _ = client
                .execute("DELETE FROM chrono_merkle_storage WHERE key = $1", &[&key])
                .await
                .map_err(|e| ChronoMerkleError::StorageError {
                    reason: format!("Failed to delete data: {}", e),
                })?;
            Ok(())
        })
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        let client = self.client()?;
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Failed to create runtime: {}", e),
            }
        })?;

        rt.block_on(async {
            let rows = client
                .query("SELECT key FROM chrono_merkle_storage ORDER BY key", &[])
                .await
                .map_err(|e| ChronoMerkleError::StorageError {
                    reason: format!("Failed to list keys: {}", e),
                })?;

            let keys = rows.iter().map(|row| row.get::<_, String>(0)).collect();
            Ok(keys)
        })
    }

    fn exists(&self, key: &str) -> Result<bool> {
        let client = self.client()?;
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Failed to create runtime: {}", e),
            }
        })?;

        rt.block_on(async {
            let count: i64 = client
                .query_one("SELECT COUNT(*) FROM chrono_merkle_storage WHERE key = $1", &[&key])
                .await
                .map_err(|e| ChronoMerkleError::StorageError {
                    reason: format!("Failed to check existence: {}", e),
                })?
                .get::<_, i64>(0);

            Ok(count > 0)
        })
    }
}

/// Redis-based storage backend with bb8 connection pooling
#[cfg(all(feature = "storage", feature = "redis-storage", not(feature = "no-std")))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct RedisStorage {
    connection_string: String,
    #[cfg(feature = "serde")]
    #[serde(skip)]
    pool: bb8_redis::bb8::Pool<bb8_redis::RedisConnectionManager>,
}

#[cfg(all(feature = "storage", feature = "redis-storage", not(feature = "no-std")))]
impl RedisStorage {
    /// Create a new Redis storage backend with connection pooling
    pub async fn new(connection_string: String) -> Result<Self> {
        let manager = bb8_redis::RedisConnectionManager::new(connection_string.clone())
            .map_err(|e| ChronoMerkleError::StorageError {
                reason: format!("Failed to create Redis connection manager: {}", e),
            })?;

        let pool = bb8_redis::bb8::Pool::builder()
            .max_size(15) // Maximum 15 connections
            .build(manager)
            .await
            .map_err(|e| ChronoMerkleError::StorageError {
                reason: format!("Failed to create Redis connection pool: {}", e),
            })?;

        Ok(Self {
            connection_string,
            pool,
        })
    }

    /// Get a connection from the pool
    async fn get_connection(&self) -> Result<bb8_redis::bb8::PooledConnection<'_, bb8_redis::RedisConnectionManager>> {
        self.pool.get().await.map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Failed to get Redis connection from pool: {}", e),
            }
        })
    }
}

#[cfg(all(feature = "storage", feature = "redis-storage", not(feature = "no-std")))]
impl StorageBackend for RedisStorage {
    fn save(&mut self, key: &str, data: &[u8]) -> Result<()> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Failed to create runtime: {}", e),
            }
        })?;

        rt.block_on(async {
            let mut conn = self.get_connection().await?;
            redis::cmd("SET")
                .arg(key)
                .arg(data)
                .query_async::<_, ()>(&mut *conn)
                .await
                .map_err(|e| ChronoMerkleError::StorageError {
                    reason: format!("Failed to save data to Redis: {}", e),
                })?;
            Ok(())
        })
    }

    fn load(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Failed to create runtime: {}", e),
            }
        })?;

        rt.block_on(async {
            let mut conn = self.get_connection().await?;
            let result: Option<Vec<u8>> = redis::cmd("GET")
                .arg(key)
                .query_async(&mut *conn)
                .await
                .map_err(|e| ChronoMerkleError::StorageError {
                    reason: format!("Failed to load data from Redis: {}", e),
                })?;

            Ok(result)
        })
    }

    fn delete(&mut self, key: &str) -> Result<()> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Failed to create runtime: {}", e),
            }
        })?;

        rt.block_on(async {
            let mut conn = self.get_connection().await?;
            redis::cmd("DEL")
                .arg(key)
                .query_async::<_, ()>(&mut *conn)
                .await
                .map_err(|e| ChronoMerkleError::StorageError {
                    reason: format!("Failed to delete data from Redis: {}", e),
                })?;
            Ok(())
        })
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Failed to create runtime: {}", e),
            }
        })?;

        rt.block_on(async {
            let mut conn = self.get_connection().await?;
            let keys: Vec<String> = redis::cmd("SCAN")
                .arg(0)
                .arg("MATCH")
                .arg("*")
                .iter_async(&mut *conn)
                .await
                .map_err(|e| ChronoMerkleError::StorageError {
                    reason: format!("Failed to scan Redis keys: {}", e),
                })?
                .collect();

            Ok(keys)
        })
    }

    fn exists(&self, key: &str) -> Result<bool> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| {
            ChronoMerkleError::StorageError {
                reason: format!("Failed to create runtime: {}", e),
            }
        })?;

        rt.block_on(async {
            let mut conn = self.get_connection().await?;
            let exists: i32 = redis::cmd("EXISTS")
                .arg(key)
                .query_async(&mut *conn)
                .await
                .map_err(|e| ChronoMerkleError::StorageError {
                    reason: format!("Failed to check key existence in Redis: {}", e),
                })?;

            Ok(exists > 0)
        })
    }
}


/// Distributed storage backend with multi-node replication
#[cfg(all(feature = "storage", feature = "distributed-storage", not(feature = "no-std")))]
#[derive(Debug)]
pub struct DistributedStorage {
    backends: Vec<Box<dyn StorageBackend + Send + Sync>>,
    replication_factor: usize,
    read_quorum: usize,
    write_quorum: usize,
}

#[cfg(all(feature = "storage", feature = "distributed-storage", not(feature = "no-std")))]
impl DistributedStorage {
    /// Create a new distributed storage backend
    pub fn new(
        backends: Vec<Box<dyn StorageBackend + Send + Sync>>,
        replication_factor: usize,
    ) -> Result<Self> {
        if backends.is_empty() {
            return Err(ChronoMerkleError::StorageError {
                reason: String::from("At least one storage backend is required"),
            });
        }

        let num_backends = backends.len();
        let replication_factor = replication_factor.min(num_backends);
        let read_quorum = (replication_factor / 2) + 1;
        let write_quorum = (replication_factor / 2) + 1;

        Ok(Self {
            backends,
            replication_factor,
            read_quorum,
            write_quorum,
        })
    }

    /// Get the backends to use for write operations (replication)
    fn write_backends(&mut self) -> &mut [Box<dyn StorageBackend + Send + Sync>] {
        &mut self.backends[..self.replication_factor]
    }

    /// Get all backends for read operations
    fn read_backends(&self) -> &[Box<dyn StorageBackend + Send + Sync>] {
        &self.backends
    }
}

#[cfg(all(feature = "storage", feature = "distributed-storage", not(feature = "no-std")))]
impl StorageBackend for DistributedStorage {
    fn save(&mut self, key: &str, data: &[u8]) -> Result<()> {
        let mut success_count = 0;
        let mut last_error = None;

        // Write to replication_factor backends
        for backend in self.write_backends() {
            match backend.as_mut().save(key, data) {
                Ok(()) => success_count += 1,
                Err(e) => last_error = Some(e),
            }
        }

        if success_count >= self.write_quorum {
            Ok(())
        } else {
            Err(last_error.unwrap_or_else(|| {
                ChronoMerkleError::StorageError {
                    reason: String::from("Write quorum not met"),
                }
            }))
        }
    }

    fn load(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut results = Vec::new();

        // Try to read from all backends
        for backend in self.read_backends() {
            match backend.as_ref().load(key) {
                Ok(Some(data)) => results.push(data),
                Ok(None) => {} // Key doesn't exist on this backend
                Err(_) => {} // Ignore read errors for now
            }
        }

        // Return the most common result if we have enough reads
        if results.len() >= self.read_quorum {
            // For simplicity, return the first result
            // In a real implementation, you'd do conflict resolution
            Ok(Some(results[0].clone()))
        } else {
            Ok(None)
        }
    }

    fn delete(&mut self, key: &str) -> Result<()> {
        let mut success_count = 0;
        let mut last_error = None;

        // Delete from replication_factor backends
        for backend in self.write_backends() {
            match backend.as_mut().delete(key) {
                Ok(()) => success_count += 1,
                Err(e) => last_error = Some(e),
            }
        }

        if success_count >= self.write_quorum {
            Ok(())
        } else {
            Err(last_error.unwrap_or_else(|| {
                ChronoMerkleError::StorageError {
                    reason: String::from("Delete quorum not met"),
                }
            }))
        }
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        let mut all_keys = BTreeMap::new();

        // Collect keys from all backends
        for backend in self.read_backends() {
            if let Ok(keys) = backend.as_ref().list_keys() {
                for key in keys {
                    *all_keys.entry(key).or_insert(0) += 1;
                }
            }
        }

        // Return keys that exist on at least read_quorum backends
        let keys: Vec<String> = all_keys
            .into_iter()
            .filter(|(_, count)| *count >= self.read_quorum)
            .map(|(key, _)| key)
            .collect();

        Ok(keys)
    }

    fn exists(&self, key: &str) -> Result<bool> {
        let mut exists_count = 0;

        // Check existence across backends
        for backend in self.read_backends() {
            match backend.as_ref().exists(key) {
                Ok(true) => exists_count += 1,
                Ok(false) => {}
                Err(_) => {} // Ignore errors for existence check
            }
        }

        Ok(exists_count >= self.read_quorum)
    }
}

#[cfg(all(test, feature = "storage"))]
mod tests {
    use super::*;
    use crate::error::ChronoMerkleError;

    #[test]
    fn test_memory_storage() {
        let mut storage = MemoryStorage::new();

        // Test save and load
        storage.save("key1", b"data1").unwrap();
        assert_eq!(storage.load("key1").unwrap(), Some(b"data1".to_vec()));

        // Test exists
        assert!(storage.exists("key1").unwrap());
        assert!(!storage.exists("key2").unwrap());

        // Test list keys
        storage.save("key2", b"data2").unwrap();
        let mut keys = storage.list_keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["key1".to_string(), "key2".to_string()]);

        // Test delete
        storage.delete("key1").unwrap();
        assert!(!storage.exists("key1").unwrap());
        assert_eq!(storage.load("key1").unwrap(), None);
    }

    #[cfg(all(feature = "file-storage", not(feature = "no-std")))]
    #[test]
    fn test_file_storage() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let mut storage = FileStorage::new(temp_dir.path().to_path_buf());

        // Test save and load
        storage.save("key1", b"data1").unwrap();
        assert_eq!(storage.load("key1").unwrap(), Some(b"data1".to_vec()));

        // Test exists
        assert!(storage.exists("key1").unwrap());
        assert!(!storage.exists("key2").unwrap());

        // Test list keys
        storage.save("key2", b"data2").unwrap();
        let mut keys = storage.list_keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["key1".to_string(), "key2".to_string()]);

        // Test delete
        storage.delete("key1").unwrap();
        assert!(!storage.exists("key1").unwrap());
        assert_eq!(storage.load("key1").unwrap(), None);
    }

    #[cfg(all(feature = "compressed-storage", feature = "std", not(feature = "no-std")))]
    #[test]
    fn test_compressed_storage() {
        let mut inner = MemoryStorage::new();
        let mut storage = CompressedStorage::new(inner, 6);

        // Test save and load with compression
        let test_data = b"This is some test data that should be compressed";
        storage.save("key1", test_data).unwrap();
        assert_eq!(storage.load("key1").unwrap(), Some(test_data.to_vec()));

        // Test exists
        assert!(storage.exists("key1").unwrap());
        assert!(!storage.exists("key2").unwrap());

        // Test list keys
        storage.save("key2", b"more data").unwrap();
        let mut keys = storage.list_keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["key1".to_string(), "key2".to_string()]);

        // Test delete
        storage.delete("key1").unwrap();
        assert!(!storage.exists("key1").unwrap());
        assert_eq!(storage.load("key1").unwrap(), None);
    }

    #[cfg(feature = "encrypted-storage")]
    #[test]
    fn test_encrypted_storage() {
        let mut inner = MemoryStorage::new();
        let key = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]; // 32 bytes
        let mut storage = EncryptedStorage::new(inner, key).unwrap();

        // Test save and load with encryption
        let test_data = b"This is sensitive data that should be encrypted";
        storage.save("key1", test_data).unwrap();
        assert_eq!(storage.load("key1").unwrap(), Some(test_data.to_vec()));

        // Test exists
        assert!(storage.exists("key1").unwrap());
        assert!(!storage.exists("key2").unwrap());

        // Test list keys
        storage.save("key2", b"more sensitive data").unwrap();
        let mut keys = storage.list_keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["key1".to_string(), "key2".to_string()]);

        // Test delete
        storage.delete("key1").unwrap();
        assert!(!storage.exists("key1").unwrap());
        assert_eq!(storage.load("key1").unwrap(), None);
    }
}