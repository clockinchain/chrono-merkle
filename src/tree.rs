//! Core ChronoMerkleTree implementation

use crate::node::Node;
pub use crate::config::TreeConfig;
#[cfg(feature = "storage")]
pub use crate::storage::{StorageBackend, TreeState};

/// ChronoMerkleTree - A time-aware Merkle tree with delta-based updates
pub struct ChronoMerkleTree<H = [u8; 32], Hasher = crate::hash::Blake3Hasher, Logger = crate::security::NoOpLogger>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: crate::hash::HashFunction<Output = H> + Sync,
    Logger: crate::security::SecurityLogger,
{
    /// All nodes stored in heap order (complete binary tree)
    /// Leaves are stored first, then internal nodes
    pub(crate) nodes: Vec<Node<H>>,
    /// Number of leaves (first N nodes are leaves)
    pub(crate) leaf_count: usize,
    /// Sparse index for timestamp lookups
    pub(crate) sparse_index: crate::sparse_index::SparseIndex,
    /// Hash function
    pub(crate) hasher: Hasher,
    /// Tree configuration
    pub(crate) config: TreeConfig,
    /// Whether to use incremental updates (vs rebuild)
    pub(crate) incremental_updates: bool,
    /// Stored delta chains for rollback capabilities
    /// Maps timestamp -> list of deltas that led to that state
    pub(crate) delta_chains: crate::sparse_index::SparseIndex,
    /// All stored delta nodes for rollback
    pub(crate) stored_deltas: Vec<Node<H>>,
    /// Security event logger
    pub(crate) security_logger: Logger,
}

#[cfg(feature = "storage")]
impl<H, Hasher, Logger> ChronoMerkleTree<H, Hasher, Logger>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: crate::hash::HashFunction<Output = H> + Sync,
    Logger: crate::security::SecurityLogger,
{
    /// Extract the current tree state for serialization
    pub fn extract_state(&self) -> crate::storage::TreeState<H> {
        crate::storage::TreeState {
            nodes: self.nodes.clone(),
            leaf_count: self.leaf_count,
            sparse_index: self.sparse_index.clone(),
            config: self.config.clone(),
            incremental_updates: self.incremental_updates,
            stored_deltas: self.stored_deltas.clone(),
            delta_chains: self.delta_chains.clone(),
        }
    }

    /// Create a tree from a previously extracted state
    pub fn from_state(
        state: crate::storage::TreeState<H>,
        hasher: Hasher,
        logger: Logger,
    ) -> Self {
        Self {
            nodes: state.nodes,
            leaf_count: state.leaf_count,
            sparse_index: state.sparse_index,
            hasher,
            config: state.config,
            incremental_updates: state.incremental_updates,
            delta_chains: state.delta_chains,
            stored_deltas: state.stored_deltas,
            security_logger: logger,
        }
    }

    /// Save the current tree state to persistent storage
    pub fn save_state(
        &self,
        storage: &mut impl crate::storage::StorageBackend,
        key: &str,
    ) -> crate::error::Result<()> {
        let state = self.extract_state();
        let serialized = serde_json::to_string(&state)
            .map_err(|e| crate::error::ChronoMerkleError::SerializationError(e.to_string()))?;
        storage.save(key, serialized.as_bytes())
    }

    /// Load a tree state from persistent storage
    pub fn load_state(
        storage: &impl crate::storage::StorageBackend,
        key: &str,
        hasher: Hasher,
        logger: Logger,
    ) -> crate::error::Result<Self> {
        let data = storage.load(key)?
            .ok_or_else(|| crate::error::ChronoMerkleError::StorageError {
                reason: format!("No data found for key: {}", key),
            })?;
        let state: crate::storage::TreeState<H> = serde_json::from_slice(&data)
            .map_err(|e| crate::error::ChronoMerkleError::DeserializationError(e.to_string()))?;
        Ok(Self::from_state(state, hasher, logger))
    }
}