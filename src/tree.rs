//! Core ChronoMerkleTree implementation

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
    pub(crate) nodes: Vec<crate::node::Node<H>>,
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
    pub(crate) stored_deltas: Vec<crate::node::Node<H>>,
    /// Security event logger
    pub(crate) security_logger: Logger,
}