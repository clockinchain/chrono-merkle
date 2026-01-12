//! Constructor methods for ChronoMerkleTree

use crate::config::TreeConfig;
use crate::error::Result;
use crate::hash::HashFunction;
use crate::security::SecurityLogger;
use crate::sparse_index::SparseIndex;
use crate::tree::ChronoMerkleTree;

#[cfg(feature = "no-std")]
use alloc::format;
#[cfg(not(feature = "no-std"))]
impl<H, Hasher, Logger> ChronoMerkleTree<H, Hasher, Logger>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: HashFunction<Output = H> + Default + Sync,
    Logger: SecurityLogger + Default,
{
    /// Create a new empty ChronoMerkleTree
    pub fn new(hasher: Hasher) -> Self
    where
        Logger: Default,
    {
        let config = TreeConfig::default();
        Self {
            nodes: Vec::new(),
            leaf_count: 0,
            sparse_index: SparseIndex::new(config.sparse_index_sparsity),
            hasher,
            config: config.clone(),
            incremental_updates: config.incremental_updates,
            delta_chains: SparseIndex::new(config.sparse_index_sparsity),
            stored_deltas: Vec::new(),
            security_logger: Logger::default(),
        }
    }

    /// Create a new empty ChronoMerkleTree with custom security logger
    pub fn with_logger(hasher: Hasher, logger: Logger) -> Self {
        let config = TreeConfig::default();
        Self {
            nodes: Vec::new(),
            leaf_count: 0,
            sparse_index: SparseIndex::new(config.sparse_index_sparsity),
            hasher,
            config: config.clone(),
            incremental_updates: config.incremental_updates,
            delta_chains: SparseIndex::new(config.sparse_index_sparsity),
            stored_deltas: Vec::new(),
            security_logger: logger,
        }
    }

    /// Create a new tree with custom configuration
    pub fn with_config(hasher: Hasher, config: TreeConfig) -> Result<Self>
    where
        Logger: Default,
    {
        config.validate()?;
        let sparse_index = SparseIndex::new(config.sparse_index_sparsity);
        let delta_chains = SparseIndex::new(config.sparse_index_sparsity);
        let tree = Self {
            nodes: Vec::new(),
            leaf_count: 0,
            sparse_index,
            hasher,
            config: config.clone(),
            incremental_updates: config.incremental_updates,
            delta_chains,
            stored_deltas: Vec::new(),
            security_logger: Logger::default(),
        };

        // Log tree initialization
        let config_summary = format!("sparsity={}, deltas={}, incremental={}, max_depth={}",
                                   config.sparse_index_sparsity,
                                   config.enable_deltas,
                                   config.incremental_updates,
                                   config.max_depth);
        let _ = tree.security_logger.log_event(&crate::security::events::tree_initialization(&config_summary));

        Ok(tree)
    }

    /// Create a new tree with custom configuration and logger
    pub fn with_config_and_logger(hasher: Hasher, config: TreeConfig, logger: Logger) -> Result<Self> {
        config.validate()?;
        let sparse_index = SparseIndex::new(config.sparse_index_sparsity);
        let delta_chains = SparseIndex::new(config.sparse_index_sparsity);
        let tree = Self {
            nodes: Vec::new(),
            leaf_count: 0,
            sparse_index,
            hasher,
            config: config.clone(),
            incremental_updates: config.incremental_updates,
            delta_chains,
            stored_deltas: Vec::new(),
            security_logger: logger,
        };

        // Log tree initialization
        let config_summary = format!("sparsity={}, deltas={}, incremental={}, max_depth={}",
                                   config.sparse_index_sparsity,
                                   config.enable_deltas,
                                   config.incremental_updates,
                                   config.max_depth);
        let _ = tree.security_logger.log_event(&crate::security::events::tree_initialization(&config_summary));

        Ok(tree)
    }
}

impl<H, Hasher> Default for ChronoMerkleTree<H, Hasher>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: HashFunction<Output = H> + Default + Sync,
{
    fn default() -> Self {
        Self::new(Hasher::default())
    }
}

impl<H, Hasher, Logger> Clone for ChronoMerkleTree<H, Hasher, Logger>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: HashFunction<Output = H> + Clone + Sync,
    Logger: SecurityLogger + Clone,
{
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            leaf_count: self.leaf_count,
            sparse_index: self.sparse_index.clone(),
            hasher: self.hasher.clone(),
            config: self.config.clone(),
            incremental_updates: self.incremental_updates,
            delta_chains: self.delta_chains.clone(),
            stored_deltas: self.stored_deltas.clone(),
            security_logger: self.security_logger.clone(),
        }
    }
}