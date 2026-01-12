//! Storage backend trait and tree state serialization


#[cfg(feature = "no-std")]
use alloc::{format, string::{String, ToString}, vec::Vec};

/// Trait for persistent storage backends
#[cfg(feature = "storage")]
pub trait StorageBackend: Send + Sync {
    /// Save data to the storage backend
    fn save(&mut self, key: &str, data: &[u8]) -> Result<(), ChronoMerkleError>;

    /// Load data from the storage backend
    fn load(&self, key: &str) -> Result<Option<Vec<u8>>, ChronoMerkleError>;

    /// Delete data from the storage backend
    fn delete(&mut self, key: &str) -> Result<(), ChronoMerkleError>;

    /// List all keys in the storage backend
    fn list_keys(&self) -> Result<Vec<String>, ChronoMerkleError>;

    /// Check if a key exists in the storage backend
    fn exists(&self, key: &str) -> Result<bool, ChronoMerkleError>;
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