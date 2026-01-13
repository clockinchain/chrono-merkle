//! Storage backend trait and tree state serialization


#[cfg(feature = "no-std")]
use alloc::{format, string::{String, ToString}, vec::Vec};

#[cfg(feature = "storage")]
use std::collections::HashMap;

#[cfg(all(feature = "storage", feature = "std", not(feature = "no-std")))]
use std::fs;
#[cfg(all(feature = "storage", feature = "std", not(feature = "no-std")))]
use std::path::PathBuf;

/// Trait for persistent storage backends
#[cfg(feature = "storage")]
pub trait StorageBackend: Send + Sync {
    /// Save data to the storage backend
    fn save(&mut self, key: &str, data: &[u8]) -> core::result::Result<(), ChronoMerkleError>;

    /// Load data from the storage backend
    fn load(&self, key: &str) -> core::result::Result<Option<Vec<u8>>, ChronoMerkleError>;

    /// Delete data from the storage backend
    fn delete(&mut self, key: &str) -> core::result::Result<(), ChronoMerkleError>;

    /// List all keys in the storage backend
    fn list_keys(&self) -> core::result::Result<Vec<String>, ChronoMerkleError>;

    /// Check if a key exists in the storage backend
    fn exists(&self, key: &str) -> core::result::Result<bool, ChronoMerkleError>;
}

/// Serializable tree state for persistence
#[cfg(feature = "storage")]
#[derive(Clone)]
pub struct TreeState<H>
where
    H: serde::Serialize + serde::de::DeserializeOwned,
{
    /// All nodes stored in heap order (complete binary tree)
    pub nodes: Vec<Node<H>>,
    /// Number of leaves (first N nodes are leaves)
    pub leaf_count: usize,
    /// Sparse index for timestamp lookups
    pub sparse_index: SparseIndex,
    /// Tree configuration
    pub config: TreeConfig,
    /// Whether to use incremental updates (vs rebuild)
    pub incremental_updates: bool,
    /// Stored delta chains for rollback capabilities
    pub stored_deltas: Vec<Node<H>>,
    /// Maps timestamp -> list of deltas that led to that state
    pub delta_chains: SparseIndex,
}

#[cfg(feature = "storage")]
impl<H> serde::Serialize for TreeState<H>
where
    H: serde::Serialize + serde::de::DeserializeOwned,
{
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("TreeState", 7)?;
        state.serialize_field("nodes", &self.nodes)?;
        state.serialize_field("leaf_count", &self.leaf_count)?;
        state.serialize_field("sparse_index", &self.sparse_index)?;
        state.serialize_field("config", &self.config)?;
        state.serialize_field("incremental_updates", &self.incremental_updates)?;
        state.serialize_field("stored_deltas", &self.stored_deltas)?;
        state.serialize_field("delta_chains", &self.delta_chains)?;
        state.end()
    }
}

#[cfg(feature = "storage")]
impl<'de, H> serde::Deserialize<'de> for TreeState<H>
where
    H: serde::Serialize + serde::de::DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(bound(deserialize = "H: serde::de::DeserializeOwned"))]
        struct TreeStateHelper<H> {
            nodes: Vec<Node<H>>,
            leaf_count: usize,
            sparse_index: SparseIndex,
            config: TreeConfig,
            incremental_updates: bool,
            stored_deltas: Vec<Node<H>>,
            delta_chains: SparseIndex,
        }

        let helper = TreeStateHelper::deserialize(deserializer)?;
        Ok(TreeState {
            nodes: helper.nodes,
            leaf_count: helper.leaf_count,
            sparse_index: helper.sparse_index,
            config: helper.config,
            incremental_updates: helper.incremental_updates,
            stored_deltas: helper.stored_deltas,
            delta_chains: helper.delta_chains,
        })
    }
}

/// In-memory storage backend for testing and temporary storage
#[cfg(feature = "storage")]
pub struct MemoryStorage {
    /// Storage map
    data: HashMap<String, Vec<u8>>,
}

#[cfg(feature = "storage")]
impl MemoryStorage {
    /// Create a new MemoryStorage instance
    pub fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
        }
    }
}

#[cfg(feature = "storage")]
impl StorageBackend for MemoryStorage {
    fn save(&mut self, key: &str, data: &[u8]) -> core::result::Result<(), ChronoMerkleError> {
        self.data.insert(key.to_string(), data.to_vec());
        Ok(())
    }

    fn load(&self, key: &str) -> core::result::Result<Option<Vec<u8>>, ChronoMerkleError> {
        Ok(self.data.get(key).cloned())
    }

    fn delete(&mut self, key: &str) -> core::result::Result<(), ChronoMerkleError> {
        self.data.remove(key);
        Ok(())
    }

    fn list_keys(&self) -> core::result::Result<Vec<String>, ChronoMerkleError> {
        Ok(self.data.keys().cloned().collect())
    }

    fn exists(&self, key: &str) -> core::result::Result<bool, ChronoMerkleError> {
        Ok(self.data.contains_key(key))
    }
}

/// File-based storage backend for persistent tree storage
#[cfg(all(feature = "storage", feature = "std", not(feature = "no-std")))]
pub struct FileStorage {
    /// Base directory for storing files
    base_dir: PathBuf,
}

#[cfg(all(feature = "storage", feature = "std", not(feature = "no-std")))]
impl FileStorage {
    /// Create a new FileStorage instance
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Get the file path for a given key
    fn get_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(format!("{}.bin", key))
    }
}

#[cfg(all(feature = "storage", feature = "std", not(feature = "no-std")))]
impl StorageBackend for FileStorage {
    fn save(&mut self, key: &str, data: &[u8]) -> core::result::Result<(), ChronoMerkleError> {
        let path = self.get_path(key);
        // Ensure the directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ChronoMerkleError::StorageError {
                reason: format!("Failed to create directory: {}", e),
            })?;
        }
        fs::write(&path, data).map_err(|e| ChronoMerkleError::StorageError {
            reason: format!("Failed to write file {}: {}", path.display(), e),
        })
    }

    fn load(&self, key: &str) -> core::result::Result<Option<Vec<u8>>, ChronoMerkleError> {
        let path = self.get_path(key);
        if path.exists() {
            fs::read(&path)
                .map(Some)
                .map_err(|e| ChronoMerkleError::StorageError {
                    reason: format!("Failed to read file {}: {}", path.display(), e),
                })
        } else {
            Ok(None)
        }
    }

    fn delete(&mut self, key: &str) -> core::result::Result<(), ChronoMerkleError> {
        let path = self.get_path(key);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| ChronoMerkleError::StorageError {
                reason: format!("Failed to delete file {}: {}", path.display(), e),
            })
        } else {
            Ok(())
        }
    }

    fn list_keys(&self) -> core::result::Result<Vec<String>, ChronoMerkleError> {
        let entries = fs::read_dir(&self.base_dir).map_err(|e| ChronoMerkleError::StorageError {
            reason: format!("Failed to read directory {}: {}", self.base_dir.display(), e),
        })?;

        let mut keys = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| ChronoMerkleError::StorageError {
                reason: format!("Failed to read directory entry: {}", e),
            })?;
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        // Remove .bin extension
                        if file_name_str.ends_with(".bin") {
                            let key = &file_name_str[..file_name_str.len() - 4];
                            keys.push(key.to_string());
                        }
                    }
                }
            }
        }
        Ok(keys)
    }

    fn exists(&self, key: &str) -> core::result::Result<bool, ChronoMerkleError> {
        Ok(self.get_path(key).exists())
    }
}