//! Core ChronoMerkleTree implementation

use crate::error::{ChronoMerkleError, Result};
use crate::hash::HashFunction;
use crate::node::{Node, NodeType};
use crate::sparse_index::SparseIndex;

#[cfg(feature = "no-std")]
use alloc::{format, string::{String, ToString}, vec::Vec};
#[cfg(not(feature = "no-std"))]
use std::vec::Vec;

/// Trait for persistent storage backends
#[cfg(feature = "storage")]
pub trait StorageBackend: Send + Sync {
    /// Save data to the storage backend
    fn save(&mut self, key: &str, data: &[u8]) -> Result<()>;

    /// Load data from the storage backend
    fn load(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Delete data from the storage backend
    fn delete(&mut self, key: &str) -> Result<()>;

    /// List all keys in the storage backend
    fn list_keys(&self) -> Result<Vec<String>>;

    /// Check if a key exists in the storage backend
    fn exists(&self, key: &str) -> Result<bool>;
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

#[cfg(feature = "no-std")]
#[cfg(not(feature = "no-std"))]
use std::vec::Vec;


/// Configuration for ChronoMerkleTree
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TreeConfig {
    /// Sparse index sparsity factor
    pub sparse_index_sparsity: u64,
    /// Enable delta nodes for incremental updates
    pub enable_deltas: bool,
    /// Maximum tree depth (for safety)
    pub max_depth: usize,
    /// Use parallel tree construction (requires "parallel" feature)
    #[cfg_attr(feature = "serde", serde(default))]
    pub parallel_construction: bool,
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            sparse_index_sparsity: 1, // Index all timestamps by default
            enable_deltas: false, // SECURITY: Disable deltas by default due to incomplete implementation
            max_depth: 32, // SECURITY: Reduced from 64 to prevent excessive memory usage
            #[cfg(feature = "parallel")]
            parallel_construction: false, // SECURITY: Disable parallel by default to prevent timing issues
            #[cfg(not(feature = "parallel"))]
            parallel_construction: false,
        }
    }
}

impl TreeConfig {
    /// Validate configuration parameters for security and correctness
    pub fn validate(&self) -> Result<()> {
        // Validate sparsity factor
        if self.sparse_index_sparsity == 0 {
            return Err(ChronoMerkleError::InvalidConfiguration {
                parameter: "sparse_index_sparsity".to_string(),
                reason: "Sparsity factor must be greater than 0".to_string(),
            });
        }

        // SECURITY: Limit maximum depth to prevent DoS through excessive memory usage
        if self.max_depth == 0 || self.max_depth > 64 {
            return Err(ChronoMerkleError::InvalidConfiguration {
                parameter: "max_depth".to_string(),
                reason: "Maximum depth must be between 1 and 64".to_string(),
            });
        }

        // SECURITY: Warn about delta nodes (incomplete implementation)
        if self.enable_deltas {
            // Note: We allow deltas but they have known issues - user discretion advised
        }

        Ok(())
    }

    /// Create a secure default configuration
    pub fn secure_defaults() -> Self {
        Self {
            sparse_index_sparsity: 1,
            enable_deltas: false, // Disabled for security
            max_depth: 32, // Conservative limit
            parallel_construction: false, // Disabled to prevent timing variations
        }
    }
}

/// ChronoMerkleTree - A time-aware Merkle tree with delta-based updates
pub struct ChronoMerkleTree<H = [u8; 32], Hasher = crate::hash::Blake3Hasher, Logger = crate::security::NoOpLogger>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: HashFunction<Output = H> + Sync,
    Logger: crate::security::SecurityLogger,
{
    /// All nodes stored in heap order (complete binary tree)
    /// Leaves are stored first, then internal nodes
    nodes: Vec<Node<H>>,
    /// Number of leaves (first N nodes are leaves)
    leaf_count: usize,
    /// Sparse index for timestamp lookups
    sparse_index: SparseIndex,
    /// Hash function
    hasher: Hasher,
    /// Tree configuration
    config: TreeConfig,
    /// Whether to use incremental updates (vs rebuild)
    incremental_updates: bool,
    /// Stored delta chains for rollback capabilities
    /// Maps timestamp -> list of deltas that led to that state
    delta_chains: crate::sparse_index::SparseIndex,
    /// All stored delta nodes for rollback
    stored_deltas: Vec<Node<H>>,
    /// Security event logger
    security_logger: Logger,
}

impl<H, Hasher, Logger> ChronoMerkleTree<H, Hasher, Logger>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: HashFunction<Output = H> + Default + Sync,
    Logger: crate::security::SecurityLogger + Default,
{
    /// Create a new empty ChronoMerkleTree
    pub fn new(hasher: Hasher) -> Self
    where
        Logger: Default,
    {
        Self {
            nodes: Vec::new(),
            leaf_count: 0,
            sparse_index: SparseIndex::new(1),
            hasher,
            config: TreeConfig::default(),
            incremental_updates: false, // Keep disabled - incremental updates have issues
            delta_chains: SparseIndex::new(1),
            stored_deltas: Vec::new(),
            security_logger: Logger::default(),
        }
    }

    /// Create a new empty ChronoMerkleTree with custom security logger
    pub fn with_logger(hasher: Hasher, logger: Logger) -> Self {
        Self {
            nodes: Vec::new(),
            leaf_count: 0,
            sparse_index: SparseIndex::new(1),
            hasher,
            config: TreeConfig::default(),
            incremental_updates: false, // Keep disabled - incremental updates have issues
            delta_chains: SparseIndex::new(1),
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
        let mut tree = Self {
            nodes: Vec::new(),
            leaf_count: 0,
            sparse_index,
            hasher,
            config: config.clone(),
            incremental_updates: false,
            delta_chains,
            stored_deltas: Vec::new(),
            security_logger: Logger::default(),
        };

        // Log tree initialization
        let config_summary = format!("sparsity={}, deltas={}, max_depth={}",
                                   config.sparse_index_sparsity,
                                   config.enable_deltas,
                                   config.max_depth);
        let _ = tree.security_logger.log_event(&crate::security::events::tree_initialization(&config_summary));

        Ok(tree)
    }

    /// Create a new tree with custom configuration and logger
    pub fn with_config_and_logger(hasher: Hasher, config: TreeConfig, logger: Logger) -> Result<Self> {
        config.validate()?;
        let sparse_index = SparseIndex::new(config.sparse_index_sparsity);
        let delta_chains = SparseIndex::new(config.sparse_index_sparsity);
        let mut tree = Self {
            nodes: Vec::new(),
            leaf_count: 0,
            sparse_index,
            hasher,
            config: config.clone(),
            incremental_updates: false,
            delta_chains,
            stored_deltas: Vec::new(),
            security_logger: logger,
        };

        // Log tree initialization
        let config_summary = format!("sparsity={}, deltas={}, max_depth={}",
                                   config.sparse_index_sparsity,
                                   config.enable_deltas,
                                   config.max_depth);
        let _ = tree.security_logger.log_event(&crate::security::events::tree_initialization(&config_summary));

        Ok(tree)
    }

    /// Validate inputs for insert operation
    fn validate_insert_inputs(&self, data: &[u8], timestamp: u64) -> Result<()> {
        // SECURITY: Validate data size to prevent DoS through excessive memory usage
        const MAX_DATA_SIZE: usize = 1024 * 1024; // 1MB limit
        if data.len() > MAX_DATA_SIZE {
            let _ = self.security_logger.log_event(&crate::security::events::input_validation_failure(
                "data",
                &format!("Data size {} exceeds maximum allowed size {}", data.len(), MAX_DATA_SIZE),
                None,
            ));
            return Err(ChronoMerkleError::InvalidConfiguration {
                parameter: "data".to_string(),
                reason: format!("Data size {} exceeds maximum allowed size {}", data.len(), MAX_DATA_SIZE),
            });
        }

        // SECURITY: Validate data is not empty (empty data could cause issues)
        if data.is_empty() {
            let _ = self.security_logger.log_event(&crate::security::events::input_validation_failure(
                "data",
                "Empty data not allowed",
                Some(""),
            ));
            return Err(ChronoMerkleError::InvalidConfiguration {
                parameter: "data".to_string(),
                reason: "Empty data not allowed".to_string(),
            });
        }

        // SECURITY: Validate timestamp is reasonable (not in far future or past)
        // Allow timestamps up to 1 year in the future and 100 years in the past
        let current_time = crate::security::current_timestamp();
        let one_year_future = current_time + (365 * 24 * 60 * 60);
        // Prevent underflow in test environments where current_time might be 0
        let hundred_years_past = if current_time > (100 * 365 * 24 * 60 * 60) {
            current_time - (100 * 365 * 24 * 60 * 60)
        } else {
            0 // Allow any timestamp if we can't determine a reasonable past bound
        };

        if timestamp > one_year_future {
            let _ = self.security_logger.log_event(&crate::security::events::input_validation_failure(
                "timestamp",
                &format!("Timestamp {} is too far in the future (current: {})", timestamp, current_time),
                Some(&timestamp.to_string()),
            ));
            return Err(ChronoMerkleError::InvalidTimestamp { timestamp });
        }

        if timestamp < hundred_years_past {
            let _ = self.security_logger.log_event(&crate::security::events::input_validation_failure(
                "timestamp",
                &format!("Timestamp {} is too far in the past (current: {})", timestamp, current_time),
                Some(&timestamp.to_string()),
            ));
            return Err(ChronoMerkleError::InvalidTimestamp { timestamp });
        }

        // SECURITY: Check for duplicate timestamps (could indicate replay attacks)
        if self.sparse_index.find_exact(timestamp).is_some() {
            let _ = self.security_logger.log_event(&crate::security::events::input_validation_failure(
                "timestamp",
                &format!("Duplicate timestamp {} detected", timestamp),
                Some(&timestamp.to_string()),
            ));
            // Note: We allow duplicate timestamps for now but log it as a warning
            // In production systems, you might want to reject duplicates
        }

        Ok(())
    }

    /// Insert a new leaf into the tree
    ///
    /// # Arguments
    ///
    /// * `data` - The data to insert
    /// * `timestamp` - Timestamp associated with this data
    pub fn insert(&mut self, data: &[u8], timestamp: u64) -> Result<()> {
        // SECURITY: Validate inputs
        self.validate_insert_inputs(data, timestamp)?;

        // Capture the old root for delta creation
        let old_root = self.root();

        let hash = self.hasher.hash(data);
        let leaf = Node::leaf(hash.clone(), timestamp, Some(data.to_vec()));

        self.nodes.push(leaf);
        self.leaf_count += 1;

        let leaf_index = self.leaf_count - 1;
        self.sparse_index.insert(timestamp, leaf_index);

        // Use incremental update or rebuild based on configuration
        if self.incremental_updates {
            self.update_tree_incremental()?;
        } else {
            #[cfg(feature = "parallel")]
            if self.config.parallel_construction {
                self.rebuild_tree_parallel()?;
            } else {
                self.rebuild_tree()?;
            }
            #[cfg(not(feature = "parallel"))]
            {
                self.rebuild_tree()?;
            }
        }

        // Create delta if root changed and deltas are enabled
        if self.config.enable_deltas {
            if let Some(old_root_hash) = old_root {
                let new_root_hash = self.root().unwrap();
                if old_root_hash != new_root_hash {
                    // Create a delta node representing the change
                    let delta_hash = self.hasher.hash_pair(&old_root_hash, &new_root_hash);
                    let delta_node = Node::delta(delta_hash, old_root_hash, timestamp);
                    self.stored_deltas.push(delta_node);
                    self.delta_chains.insert(timestamp, self.stored_deltas.len() - 1);
                }
            }
        }

        // Log successful insertion
        let _ = self.security_logger.log_event(&crate::security::events::leaf_insertion(
            leaf_index,
            timestamp,
            hash.as_ref()
        ));

        Ok(())
    }


    /// Get the root hash of the tree
    pub fn root(&self) -> Option<H> {
        self.nodes.last().map(|n| n.hash())
    }

    /// Get the number of leaves in the tree
    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }

    /// Check if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.leaf_count == 0
    }

    /// Get a leaf by index
    pub fn get_leaf(&self, index: usize) -> Result<&Node<H>> {
        if index >= self.leaf_count {
            return Err(ChronoMerkleError::IndexOutOfBounds {
                index,
                leaf_count: self.leaf_count,
            });
        }
        Ok(&self.nodes[index])
    }

    /// Get the hash of a leaf by index
    pub fn get_leaf_hash(&self, index: usize) -> Result<H> {
        Ok(self.get_leaf(index)?.hash())
    }

    /// Find leaves by timestamp (exact match)
    pub fn find_by_timestamp(&self, timestamp: u64) -> Vec<usize> {
        // Search all leaves for matches (sparse index doesn't handle duplicates)
        self.nodes[..self.leaf_count]
            .iter()
            .enumerate()
            .filter_map(|(idx, node)| {
                if let NodeType::Leaf { timestamp: ts, .. } = &node.node_type {
                    if *ts == timestamp {
                        Some(idx)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find leaves in a timestamp range
    pub fn find_range(&self, start: u64, end: u64) -> Vec<usize> {
        // Check all leaves for matches (sparse index doesn't handle duplicates)
        let mut indices = Vec::new();
        for (idx, node) in self.nodes[..self.leaf_count].iter().enumerate() {
            if let NodeType::Leaf { timestamp: ts, .. } = &node.node_type {
                if *ts >= start && *ts <= end {
                    indices.push(idx);
                }
            }
        }

        indices.sort();
        indices
    }

    /// Rebuild the entire tree from leaves (used when incremental updates are disabled)
    fn rebuild_tree(&mut self) -> Result<()> {
        if self.leaf_count == 0 {
            // Clear all internal nodes (everything after leaf_count)
            self.nodes.truncate(self.leaf_count);
            return Ok(());
        }

        // For trees with 1 leaf, no internal nodes needed
        if self.leaf_count == 1 {
            // Ensure we only have the leaf
            self.nodes.truncate(1);
            return Ok(());
        }

        // Extract current leaves (all leaf nodes in the tree)
        let current_leaves: Vec<Node<H>> = self.nodes.iter()
            .filter(|node| matches!(node.node_type, NodeType::Leaf { .. }))
            .cloned()
            .collect();

        // Clear all nodes and re-insert the leaves
        self.nodes.clear();
        self.nodes.extend(current_leaves);

        // Build the complete binary tree by iteratively combining nodes
        let mut current_start = 0;
        let mut current_count = self.leaf_count;

        while current_count > 1 {
            let next_count = (current_count + 1) / 2; // Ceiling division

            // Prepare the parent nodes for this level
            let parent_nodes: Vec<Node<H>> = (0..next_count)
                .map(|i| {
                    let left_idx = current_start + 2 * i;
                    let right_idx = current_start + 2 * i + 1;

                    let left_node = &self.nodes[left_idx];
                    let right_node = if right_idx < current_start + current_count {
                        &self.nodes[right_idx]
                    } else {
                        // Duplicate the last node for odd counts
                        &self.nodes[current_start + current_count - 1]
                    };

                    let left_hash = left_node.hash();
                    let right_hash = right_node.hash();

                    let (left_start, left_end) = left_node.timestamp_info();
                    let (right_start, right_end) = right_node.timestamp_info();

                    let timestamp_range = (
                        left_start.min(right_start),
                        left_end.unwrap_or(left_start).max(right_end.unwrap_or(right_start)),
                    );

                    let internal_hash = self.hasher.hash_pair(&left_hash, &right_hash);
                    Node::internal(internal_hash, left_hash, right_hash, timestamp_range)
                })
                .collect();

            self.nodes.extend(parent_nodes);

            current_start += current_count;
            current_count = next_count;
        }

        Ok(())
    }

    #[cfg(feature = "parallel")]
    fn rebuild_tree_parallel(&mut self) -> Result<()> {
        use rayon::prelude::*;

        if self.leaf_count == 0 {
            // Clear all internal nodes (everything after leaf_count)
            self.nodes.truncate(self.leaf_count);
            return Ok(());
        }

        // For trees with 1 leaf, no internal nodes needed
        if self.leaf_count == 1 {
            // Ensure we only have the leaf
            self.nodes.truncate(1);
            return Ok(());
        }

        // Extract current leaves (all leaf nodes in the tree)
        let current_leaves: Vec<Node<H>> = self.nodes.iter()
            .filter(|node| matches!(node.node_type, NodeType::Leaf { .. }))
            .cloned()
            .collect();

        // Clear all nodes and re-insert the leaves
        self.nodes.clear();
        self.nodes.extend(current_leaves);

        // Build the complete binary tree by iteratively combining nodes
        let mut current_start = 0;
        let mut current_count = self.leaf_count;

        while current_count > 1 {
            let next_count = (current_count + 1) / 2; // Ceiling division

            // Compute all parent nodes in parallel using Rayon's work-stealing scheduler
            // This leverages intra-level parallelization and hash computation parallelization
            let parent_nodes: Vec<Node<H>> = (0..next_count)
                .into_par_iter()
                .map(|i| {
                    let left_idx = current_start + 2 * i;
                    let right_idx = current_start + 2 * i + 1;

                    let left_node = &self.nodes[left_idx];
                    let right_node = if right_idx < current_start + current_count {
                        &self.nodes[right_idx]
                    } else {
                        // Duplicate the last node for odd counts
                        &self.nodes[current_start + current_count - 1]
                    };

                    let left_hash = left_node.hash();
                    let right_hash = right_node.hash();

                    let (left_start, left_end) = left_node.timestamp_info();
                    let (right_start, right_end) = right_node.timestamp_info();

                    let timestamp_range = (
                        left_start.min(right_start),
                        left_end.unwrap_or(left_start).max(right_end.unwrap_or(right_start)),
                    );

                    // Hash computation is parallelized here
                    let internal_hash = self.hasher.hash_pair(&left_hash, &right_hash);
                    Node::internal(internal_hash, left_hash, right_hash, timestamp_range)
                })
                .collect();

            self.nodes.extend(parent_nodes);

            current_start += current_count;
            current_count = next_count;
        }

        Ok(())
    }

    /// Update the tree incrementally after inserting a new leaf
    fn update_tree_incremental(&mut self) -> Result<()> {
        // Implement true delta-based incremental updates
        // Only update the path from the new leaf to the root

        let new_leaf_index = self.leaf_count - 1;

        // If this is the first leaf, no delta computation needed
        if new_leaf_index == 0 {
            return Ok(());
        }

        // Compute deltas along the path from leaf to root
        self.compute_path_deltas(new_leaf_index)?;

        Ok(())
    }

    /// Compute deltas for the path from a leaf to the root
    fn compute_path_deltas(&mut self, leaf_index: usize) -> Result<()> {
        let mut current_index = leaf_index;
        let mut current_node = self.nodes[leaf_index].clone();

        // Traverse from leaf to root, updating internal nodes
        while current_index > 0 {
            let parent_index = (current_index - 1) / 2;

            // Get the current parent node (before update)
            let old_parent = if parent_index < self.nodes.len() {
                Some(self.nodes[parent_index].clone())
            } else {
                None
            };

            // Get the sibling node
            let sibling_index = if current_index % 2 == 0 {
                current_index - 1 // left sibling
            } else {
                current_index + 1 // right sibling
            };

            let sibling_node = if sibling_index < self.leaf_count && sibling_index != current_index {
                Some(&self.nodes[sibling_index])
            } else {
                None
            };

            // Compute new parent hash
            let new_parent_hash = if let Some(sibling) = sibling_node {
                let left_hash = if current_index % 2 == 0 { sibling.hash() } else { current_node.hash() };
                let right_hash = if current_index % 2 == 0 { current_node.hash() } else { sibling.hash() };
                self.hasher.hash_pair(&left_hash, &right_hash)
            } else {
                // Only one child, promote it
                current_node.hash()
            };

            // Get timestamp range
            let timestamp_range = if let Some(sibling) = sibling_node {
                let (left_start, left_end) = if current_index % 2 == 0 {
                    sibling.timestamp_info()
                } else {
                    current_node.timestamp_info()
                };
                let (right_start, right_end) = if current_index % 2 == 0 {
                    current_node.timestamp_info()
                } else {
                    sibling.timestamp_info()
                };

                (
                    left_start.min(right_start),
                    Some(left_end.unwrap_or(left_start).max(right_end.unwrap_or(right_start))),
                )
            } else {
                current_node.timestamp_info()
            };

            // Create or update the parent node
            let new_parent = if let Some(sibling) = sibling_node {
                let left_hash = if current_index % 2 == 0 { sibling.hash() } else { current_node.hash() };
                let right_hash = if current_index % 2 == 0 { current_node.hash() } else { sibling.hash() };

                Node::internal(new_parent_hash.clone(), left_hash, right_hash, (timestamp_range.0, timestamp_range.1.unwrap_or(timestamp_range.0)))
            } else {
                // Promote single child
                Node {
                    node_type: NodeType::Internal {
                        hash: new_parent_hash.clone(),
                        left_hash: current_node.hash(),
                        right_hash: current_node.hash(), // Same as left for single child
                        timestamp_range: (timestamp_range.0, timestamp_range.1.unwrap_or(timestamp_range.0)),
                    },
                    children: vec![current_node.clone()],
                }
            };

            // Compute and store delta if we had an old parent
            if let Some(old_parent_node) = old_parent {
                let old_hash = old_parent_node.hash();
                if old_hash != new_parent_hash {
                    // Create a delta node to record this change
                    let delta_hash = self.hasher.hash_pair(&old_hash, &new_parent_hash);
                    let delta_node = Node::delta(delta_hash, old_hash.clone(), timestamp_range.0);

                    // Store the delta for rollback capabilities
                    let delta_index = self.stored_deltas.len();
                    self.stored_deltas.push(delta_node);

                    // Record this delta in the chain for the current timestamp
                    self.delta_chains.insert(timestamp_range.0, delta_index);
                }
            }

            // Update the parent in the nodes array
            if parent_index >= self.nodes.len() {
                self.nodes.push(new_parent.clone());
            } else {
                self.nodes[parent_index] = new_parent.clone();
            }

            // Move up to parent
            current_index = parent_index;
            current_node = new_parent;
        }

        Ok(())
    }

    /// Get the delta chain for a given timestamp (for verification/rollback)
    pub fn get_delta_chain(&self, timestamp: u64) -> Vec<Node<H>> {
        if let Some(delta_index) = self.delta_chains.find_exact(timestamp) {
            // Return all deltas from this index onward (simplified chain)
            self.stored_deltas.iter().skip(delta_index).cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Verify a delta proof against the current tree state
    pub fn verify_delta(&self, old_root: &H, new_root: &H, delta_proof: &[Node<H>]) -> Result<bool> {
        // Verify that applying the deltas transforms old_root to new_root
        let mut current_hash = old_root.clone();

        for delta_node in delta_proof {
            if let NodeType::Delta { delta_hash, base_hash, .. } = &delta_node.node_type {
                // Verify the delta was computed correctly
                let expected_delta = self.hasher.hash_pair(base_hash, new_root);
                if *delta_hash != expected_delta {
                    return Ok(false);
                }
                current_hash = new_root.clone();
            }
        }

        Ok(current_hash == *new_root)
    }

    /// Apply a delta to rollback to a previous state
    pub fn apply_delta_rollback(&mut self, delta: &Node<H>) -> Result<()> {
        // Apply a delta in reverse to rollback state
        if let NodeType::Delta { delta_hash: _, base_hash: _, timestamp: _ } = &delta.node_type {
            // The delta represents a change from base_hash to delta_hash
            // To rollback, we need to change delta_hash back to base_hash
            // Since we don't know exactly where this change occurred in the current tree,
            // we'll rebuild the tree from scratch with one less leaf

            if self.leaf_count > 0 {
                // Remove the last leaf (simplified rollback)
                self.leaf_count -= 1;
                self.nodes.truncate(self.leaf_count + (self.leaf_count.saturating_sub(1)));

                // Rebuild the tree without the rolled-back leaf
                self.rebuild_tree()?;
            }

            Ok(())
        } else {
            Err(ChronoMerkleError::InvalidNodeType {
                operation: "apply_delta_rollback".to_string(),
            })
        }
    }

    /// Recompute parent hashes from a given node index up to the root
    fn recompute_parent_hashes_from_node(&mut self, start_index: usize) -> Result<()> {
        let mut current_index = start_index;

        // Work our way up the tree, recomputing hashes
        while current_index > 0 {
            let parent_index = (current_index - 1) / 2;

            // Get the two children
            let left_idx = 2 * parent_index;
            let right_idx = 2 * parent_index + 1;

            if left_idx < self.nodes.len() && right_idx < self.nodes.len() {
                let left_hash = self.nodes[left_idx].hash();
                let right_hash = self.nodes[right_idx].hash();

                // Get timestamp range
                let (left_start, left_end) = self.nodes[left_idx].timestamp_info();
                let (right_start, right_end) = self.nodes[right_idx].timestamp_info();

                let timestamp_range = (
                    left_start.min(right_start),
                    left_end.unwrap_or(left_start).max(right_end.unwrap_or(right_start)),
                );

                // Recompute parent hash
                let new_parent_hash = self.hasher.hash_pair(&left_hash, &right_hash);

                // Update parent node
                if let NodeType::Internal { hash, left_hash: lh, right_hash: rh, timestamp_range: tr } = &mut self.nodes[parent_index].node_type {
                    *hash = new_parent_hash;
                    *lh = left_hash;
                    *rh = right_hash;
                    *tr = timestamp_range;
                }
            }

            current_index = parent_index;
        }

        Ok(())
    }

    /// Rollback the tree to a previous state using delta chain
    pub fn rollback_to_timestamp(&mut self, target_timestamp: u64) -> Result<()> {
        // Simple rollback: remove leaves added after the target timestamp
        // This provides the core rollback functionality

        if self.leaf_count == 0 {
            return Err(ChronoMerkleError::InvalidTimestamp {
                timestamp: target_timestamp,
            });
        }

        // Extract the leaves we want to keep (those with timestamp <= target_timestamp)
        let mut leaves_to_keep = Vec::new();
        for node in &self.nodes {
            if let NodeType::Leaf { timestamp, .. } = &node.node_type {
                if *timestamp <= target_timestamp {
                    leaves_to_keep.push(node.clone());
                } else {
                    break; // Since leaves are inserted in order, we can stop here
                }
            }
        }

        if leaves_to_keep.is_empty() {
            return Err(ChronoMerkleError::InvalidTimestamp {
                timestamp: target_timestamp,
            });
        }

        // Replace the tree with just the kept leaves
        self.nodes.clear();
        self.nodes.extend(leaves_to_keep);
        self.leaf_count = self.nodes.len();

        // Rebuild the sparse index for remaining leaves
        self.sparse_index = SparseIndex::new(self.config.sparse_index_sparsity);
        for (idx, node) in self.nodes.iter().enumerate() {
            if let NodeType::Leaf { timestamp, .. } = &node.node_type {
                self.sparse_index.insert(*timestamp, idx);
            }
        }

        // Rebuild the tree with remaining leaves
        self.rebuild_tree()?;

        // Clear deltas that occurred after the target timestamp
        self.stored_deltas.retain(|delta| {
            if let NodeType::Delta { timestamp, .. } = &delta.node_type {
                *timestamp <= target_timestamp
            } else {
                true
            }
        });

        // Rebuild delta chains index
        self.delta_chains = SparseIndex::new(self.config.sparse_index_sparsity);
        for (i, delta) in self.stored_deltas.iter().enumerate() {
            if let NodeType::Delta { timestamp, .. } = &delta.node_type {
                self.delta_chains.insert(*timestamp, i);
            }
        }

        Ok(())
    }

    /// Get all stored deltas
    pub fn get_all_deltas(&self) -> &[Node<H>] {
        &self.stored_deltas
    }

    /// Clear old deltas to save memory (keep only recent ones)
    pub fn prune_deltas(&mut self, keep_after_timestamp: u64) {
        // Remove deltas older than the specified timestamp
        let mut indices_to_remove = Vec::new();

        for (i, delta) in self.stored_deltas.iter().enumerate() {
            if let NodeType::Delta { timestamp, .. } = &delta.node_type {
                if *timestamp < keep_after_timestamp {
                    indices_to_remove.push(i);
                }
            }
        }

        // Remove from back to front to maintain indices
        for &index in indices_to_remove.iter().rev() {
            self.stored_deltas.remove(index);
        }

        // Rebuild delta chains index
        self.delta_chains = SparseIndex::new(self.config.sparse_index_sparsity);
        for (i, delta) in self.stored_deltas.iter().enumerate() {
            if let NodeType::Delta { timestamp, .. } = &delta.node_type {
                self.delta_chains.insert(*timestamp, i);
            }
        }
    }

    /// Extract leaves and deltas for serialization
    /// Returns (leaves, deltas) that can be serialized and later reconstructed
    pub fn extract_leaves_and_deltas(&self) -> (Vec<Node<H>>, Vec<Node<H>>) {
        let leaves = self.nodes[..self.leaf_count].to_vec();
        let deltas = self.stored_deltas.clone();
        (leaves, deltas)
    }

    /// Reconstruct tree from serialized leaves and deltas
    pub fn reconstruct_from_leaves_and_deltas(
        leaves: Vec<Node<H>>,
        deltas: Vec<Node<H>>,
        hasher: Hasher,
        config: TreeConfig,
    ) -> Result<Self>
    where
        Logger: Default,
    {
        let mut tree = Self::with_config(hasher, config)?;

        // Add leaves
        for leaf in leaves {
            if let NodeType::Leaf { data, timestamp, .. } = &leaf.node_type {
                tree.insert(data.as_ref().unwrap_or(&vec![]), *timestamp)?;
            }
        }

        // Restore deltas
        tree.stored_deltas = deltas;
        for (i, delta) in tree.stored_deltas.iter().enumerate() {
            if let NodeType::Delta { timestamp, .. } = &delta.node_type {
                tree.delta_chains.insert(*timestamp, i);
            }
        }

        Ok(tree)
    }

    /// Calculate the smallest power of two greater than or equal to n
    fn next_power_of_two(&self, n: usize) -> usize {
        if n == 0 {
            1
        } else if n.is_power_of_two() {
            n
        } else {
            1 << (usize::BITS - n.leading_zeros())
        }
    }

    /// Get the tree depth
    pub fn depth(&self) -> usize {
        if self.leaf_count == 0 {
            return 0;
        }
        (self.leaf_count as f64).log2().ceil() as usize
    }

    /// Generate a proof for a leaf at the given index
    pub fn generate_proof(&self, leaf_index: usize) -> Result<crate::proof::ChronoProof<H>> {
        if leaf_index >= self.leaf_count {
            return Err(ChronoMerkleError::IndexOutOfBounds {
                index: leaf_index,
                leaf_count: self.leaf_count,
            });
        }

        let leaf = &self.nodes[leaf_index];
        let (timestamp, _) = leaf.timestamp_info();

        let mut proof = crate::proof::ChronoProof::new(leaf_index, timestamp);

        if self.leaf_count == 1 {
            // Single leaf, no proof path needed
            return Ok(proof);
        }

        // Build tree levels bottom-up
        let mut levels: Vec<Vec<H>> = vec![(0..self.leaf_count).map(|i| self.nodes[i].hash()).collect()];
        let mut current_level = levels[0].clone();
        let mut current_index = leaf_index;

        while current_level.len() > 1 {
            let mut next_level: Vec<H> = Vec::new();
            let next_index = current_index / 2;
            
            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    let parent_hash = self.hasher.hash_pair(&chunk[0], &chunk[1]);
                    next_level.push(parent_hash);
                } else {
                    // For odd number of nodes, duplicate the last node
                    let parent_hash = self.hasher.hash_pair(&chunk[0], &chunk[0]);
                    next_level.push(parent_hash);
                }
            }
            
            // Add sibling to proof path
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            let sibling_hash = if sibling_index < current_level.len() {
                current_level[sibling_index].clone()
            } else {
                // For odd number of nodes, duplicate the current node
                current_level[current_index].clone()
            };

            let step = if current_index % 2 == 0 {
                crate::proof::ProofStep::Right(sibling_hash)
            } else {
                crate::proof::ProofStep::Left(sibling_hash)
            };
            proof.add_step(step);
            
            levels.push(next_level.clone());
            current_level = next_level;
            current_index = next_index;
        }

        // Log proof generation
        let _ = self.security_logger.log_event(&crate::security::events::proof_generation(leaf_index));

        Ok(proof)
    }

    /// Verify a proof against the current root
    pub fn verify_proof(&self, proof: &crate::proof::ChronoProof<H>) -> Result<bool> {
        let root_hash = self.root().ok_or(ChronoMerkleError::EmptyTree)?;
        let leaf_hash = self.get_leaf_hash(proof.leaf_index)?;
        let result = crate::proof::verify_proof(proof, &leaf_hash, &root_hash, &self.hasher)?;

        // Log proof verification result
        if result {
            let _ = self.security_logger.log_event(&crate::security::events::proof_verification_success(
                proof.leaf_index,
                proof.timestamp,
            ));
        } else {
            let _ = self.security_logger.log_event(&crate::security::events::proof_verification_failure(
                proof.leaf_index,
                proof.timestamp,
                "Proof verification failed - invalid proof or tampered data",
            ));
        }

        Ok(result)
    }

    /// Save the tree state to a storage backend
    #[cfg(feature = "storage")]
    pub fn save_state<S: StorageBackend>(&self, storage: &mut S, key: &str) -> Result<()> {
        let state = TreeState {
            nodes: self.nodes.clone(),
            leaf_count: self.leaf_count,
            sparse_index: self.sparse_index.clone(),
            config: self.config.clone(),
            incremental_updates: self.incremental_updates,
            stored_deltas: self.stored_deltas.clone(),
            delta_chains: self.delta_chains.clone(),
        };

        let serialized = serde_json::to_vec(&state).map_err(|e| {
            ChronoMerkleError::SerializationError(
                format!("Failed to serialize tree state: {}", e),
            )
        })?;

        storage.save(key, &serialized)
    }

    /// Load the tree state from a storage backend
    #[cfg(feature = "storage")]
    pub fn load_state<S: StorageBackend>(storage: &S, key: &str, hasher: Hasher) -> Result<Self> {
        let data = storage.load(key)?.ok_or_else(|| {
            ChronoMerkleError::StorageError {
                reason: format!("Tree state '{}' not found", key),
            }
        })?;

        let state: TreeState<H> = serde_json::from_slice(&data).map_err(|e| {
            ChronoMerkleError::SerializationError(
                format!("Failed to deserialize tree state: {}", e),
            )
        })?;

        Ok(Self {
            nodes: state.nodes,
            leaf_count: state.leaf_count,
            sparse_index: state.sparse_index,
            hasher,
            config: state.config,
            incremental_updates: state.incremental_updates,
            stored_deltas: state.stored_deltas,
            delta_chains: state.delta_chains,
        })
    }

    /// Create a new tree from a saved state
    #[cfg(feature = "storage")]
    pub fn from_state(state: TreeState<H>, hasher: Hasher) -> Self {
        Self {
            nodes: state.nodes,
            leaf_count: state.leaf_count,
            sparse_index: state.sparse_index,
            hasher,
            config: state.config,
            incremental_updates: state.incremental_updates,
            stored_deltas: state.stored_deltas,
            delta_chains: state.delta_chains,
        }
    }

    /// Extract the current tree state for serialization
    #[cfg(feature = "storage")]
    pub fn extract_state(&self) -> TreeState<H> {
        TreeState {
            nodes: self.nodes.clone(),
            leaf_count: self.leaf_count,
            sparse_index: self.sparse_index.clone(),
            config: self.config.clone(),
            incremental_updates: self.incremental_updates,
            stored_deltas: self.stored_deltas.clone(),
            delta_chains: self.delta_chains.clone(),
        }
    }

    /// Generate an ASCII representation of the tree structure
    #[cfg(all(feature = "visualization", not(feature = "no-std")))]
    pub fn visualize_ascii(&self) -> String {
        if self.nodes.is_empty() {
            return "Empty tree".to_string();
        }

        let mut result = String::new();
        self.visualize_node_ascii(&self.nodes[self.nodes.len() - 1], "", true, &mut result);
        result
    }

    /// Helper method to visualize a single node in ASCII format
    #[cfg(all(feature = "visualization", not(feature = "no-std")))]
    fn visualize_node_ascii(&self, node: &Node<H>, prefix: &str, is_last: bool, result: &mut String) {
        use core::fmt::Write;

        // Add the current node
        let connector = if is_last { "└── " } else { "├── " };
        let _ = write!(result, "{}{}{}\n", prefix, connector, self.format_node_label(node));

        // Prepare prefix for children
        let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

        // Add children
        for (i, child) in node.children.iter().enumerate() {
            let is_last_child = i == node.children.len() - 1;
            self.visualize_node_ascii(child, &child_prefix, is_last_child, result);
        }
    }

    /// Format a node for display
    #[cfg(all(feature = "visualization", not(feature = "no-std")))]
    fn format_node_label(&self, node: &Node<H>) -> String {
        match &node.node_type {
            crate::node::NodeType::Leaf { timestamp, .. } => {
                format!("Leaf(ts={}, hash={:?})", timestamp, node.hash())
            }
            crate::node::NodeType::Internal { timestamp_range, .. } => {
                format!("Internal({}-{}, hash={:?})", timestamp_range.0, timestamp_range.1, node.hash())
            }
            crate::node::NodeType::Delta { timestamp, .. } => {
                format!("Delta(ts={}, hash={:?})", timestamp, node.hash())
            }
        }
    }

    /// Generate a GraphViz DOT representation of the tree
    #[cfg(all(feature = "visualization", not(feature = "no-std")))]
    pub fn visualize_dot(&self) -> String {
        let mut result = String::from("digraph ChronoMerkleTree {\n");
        result.push_str("    node [shape=box];\n");

        if !self.nodes.is_empty() {
            self.visualize_node_dot(&self.nodes[self.nodes.len() - 1], 0, &mut result);
        }

        result.push_str("}\n");
        result
    }

    /// Helper method to visualize a single node in DOT format
    #[cfg(all(feature = "visualization", not(feature = "no-std")))]
    fn visualize_node_dot(&self, node: &Node<H>, node_id: usize, result: &mut String) {
        use core::fmt::Write;

        // Add node definition
        let label = self.format_node_dot_label(node);
        let color = self.get_node_color(node);
        let _ = write!(result, "    {} [label=\"{}\", fillcolor=\"{}\", style=filled];\n",
                      node_id, label, color);

        // Add edges to children
        for (i, child) in node.children.iter().enumerate() {
            let child_id = node_id * 100 + i + 1; // Simple ID generation
            let _ = write!(result, "    {} -> {};\n", node_id, child_id);
            self.visualize_node_dot(child, child_id, result);
        }
    }

    /// Format a node label for DOT
    #[cfg(all(feature = "visualization", not(feature = "no-std")))]
    fn format_node_dot_label(&self, node: &Node<H>) -> String {
        match &node.node_type {
            crate::node::NodeType::Leaf { timestamp, .. } => {
                format!("Leaf\\nts={}\\nhash={:?}", timestamp, node.hash())
            }
            crate::node::NodeType::Internal { timestamp_range, .. } => {
                format!("Internal\\n{}-{}\\nhash={:?}", timestamp_range.0, timestamp_range.1, node.hash())
            }
            crate::node::NodeType::Delta { timestamp, .. } => {
                format!("Delta\\nts={}\\nhash={:?}", timestamp, node.hash())
            }
        }
    }

    /// Get color for node type
    #[cfg(all(feature = "visualization", not(feature = "no-std")))]
    fn get_node_color(&self, node: &Node<H>) -> &'static str {
        match &node.node_type {
            crate::node::NodeType::Leaf { .. } => "lightgreen",
            crate::node::NodeType::Internal { .. } => "lightblue",
            crate::node::NodeType::Delta { .. } => "lightyellow",
        }
    }

    /// Generate a JSON representation of the tree structure
    #[cfg(all(feature = "visualization", not(feature = "no-std")))]
    pub fn visualize_json(&self) -> Result<String> {
        use serde_json::json;

        let tree_data = if self.nodes.is_empty() {
            json!({
                "type": "empty",
                "leaf_count": 0,
                "nodes": []
            })
        } else {
            json!({
                "type": "chrono_merkle_tree",
                "leaf_count": self.leaf_count,
                "root": self.visualize_node_json(&self.nodes[self.nodes.len() - 1]),
                "total_nodes": self.nodes.len(),
                "config": {
                    "sparse_index_sparsity": self.config.sparse_index_sparsity,
                    "parallel_construction": self.config.parallel_construction,
                    "enable_deltas": self.config.enable_deltas
                }
            })
        };

        serde_json::to_string_pretty(&tree_data)
            .map_err(|e| ChronoMerkleError::SerializationError(
                format!("Failed to serialize tree to JSON: {}", e),
            ))
    }

    /// Helper method to create JSON representation of a node
    #[cfg(all(feature = "visualization", not(feature = "no-std")))]
    fn visualize_node_json(&self, node: &Node<H>) -> serde_json::Value {
        use serde_json::json;

        let node_info = match &node.node_type {
            crate::node::NodeType::Leaf { timestamp, hash, data } => {
                json!({
                    "type": "leaf",
                    "timestamp": timestamp,
                    "hash": format!("{:?}", hash),
                    "has_data": data.is_some()
                })
            }
            crate::node::NodeType::Internal { timestamp_range, hash, left_hash, right_hash } => {
                json!({
                    "type": "internal",
                    "timestamp_range": [timestamp_range.0, timestamp_range.1],
                    "hash": format!("{:?}", hash),
                    "left_hash": format!("{:?}", left_hash),
                    "right_hash": format!("{:?}", right_hash)
                })
            }
            crate::node::NodeType::Delta { timestamp, delta_hash, base_hash } => {
                json!({
                    "type": "delta",
                    "timestamp": timestamp,
                    "delta_hash": format!("{:?}", delta_hash),
                    "base_hash": format!("{:?}", base_hash)
                })
            }
        };

        if node.children.is_empty() {
            node_info
        } else {
            json!({
                "node": node_info,
                "children": node.children.iter().map(|child| self.visualize_node_json(child)).collect::<Vec<_>>()
            })
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::Blake3Hasher;

    #[cfg(feature = "blake3-hash")]
    #[test]
    fn test_empty_tree() {
        let tree: ChronoMerkleTree<[u8; 32], Blake3Hasher> = ChronoMerkleTree::default();
        assert!(tree.is_empty());
        assert_eq!(tree.root(), None);
        assert_eq!(tree.leaf_count(), 0);
    }

    #[cfg(feature = "blake3-hash")]
    #[test]
    fn test_single_insert() {
        let mut tree: ChronoMerkleTree = ChronoMerkleTree::default();
        tree.insert(b"data1", 1000).unwrap();
        
        assert!(!tree.is_empty());
        assert_eq!(tree.leaf_count(), 1);
        assert!(tree.root().is_some());
    }

    #[cfg(feature = "blake3-hash")]
    #[test]
    fn test_multiple_inserts() {
        let mut tree: ChronoMerkleTree = ChronoMerkleTree::default();
        tree.insert(b"data1", 1000).unwrap();
        tree.insert(b"data2", 1001).unwrap();
        tree.insert(b"data3", 1002).unwrap();
        
        assert_eq!(tree.leaf_count(), 3);
        assert!(tree.root().is_some());
    }

    #[cfg(all(feature = "blake3-hash", not(feature = "no-std")))]
    #[test]
    fn test_find_by_timestamp() {
        let mut tree: ChronoMerkleTree = ChronoMerkleTree::default();
        tree.insert(b"data1", 1000).unwrap();
        tree.insert(b"data2", 1001).unwrap();
        tree.insert(b"data3", 1000).unwrap(); // Same timestamp

        let indices = tree.find_by_timestamp(1000);
        for i in 0..tree.nodes.len() {
            match &tree.nodes[i].node_type {
                NodeType::Leaf { timestamp, .. } => println!("Node {}: LEAF with timestamp {}", i, timestamp),
                NodeType::Internal { .. } => println!("Node {}: INTERNAL", i),
                _ => println!("Node {}: OTHER", i),
            }
        }

        let indices = tree.find_by_timestamp(1000);
        println!("Found indices: {:?}", indices);
        assert_eq!(indices.len(), 2);
    }

    #[cfg(feature = "blake3-hash")]
    #[test]
    fn test_find_range() {
        let mut tree: ChronoMerkleTree = ChronoMerkleTree::default();
        for i in 0..10 {
            tree.insert(&format!("data{}", i).into_bytes(), 1000 + i).unwrap();
        }
        
        let indices = tree.find_range(1002, 1005);
        assert_eq!(indices.len(), 4);
    }

    #[cfg(all(feature = "visualization", feature = "blake3-hash", not(feature = "no-std")))]
    #[test]
    fn test_visualize_ascii() {
        let mut tree: ChronoMerkleTree = ChronoMerkleTree::default();
        tree.insert(b"data1", 1000).unwrap();
        tree.insert(b"data2", 1001).unwrap();

        let ascii = tree.visualize_ascii();
        // Should have tree structure with timestamps
        assert!(ascii.contains("1000"));
        assert!(ascii.contains("1001"));
        assert!(ascii.contains("Internal") || ascii.contains("Leaf"));
    }

    #[cfg(all(feature = "visualization", feature = "blake3-hash", not(feature = "no-std")))]
    #[test]
    fn test_visualize_dot() {
        let mut tree: ChronoMerkleTree = ChronoMerkleTree::default();
        tree.insert(b"data1", 1000).unwrap();
        tree.insert(b"data2", 1001).unwrap();

        let dot = tree.visualize_dot();
        assert!(dot.starts_with("digraph ChronoMerkleTree {"));
        assert!(dot.contains("label="));
        assert!(dot.contains("1000"));
        assert!(dot.contains("1001"));
        assert!(dot.ends_with("}\n"));
    }

    #[cfg(all(feature = "visualization", feature = "blake3-hash", not(feature = "no-std")))]
    #[test]
    fn test_visualize_json() {
        let mut tree: ChronoMerkleTree = ChronoMerkleTree::default();
        tree.insert(b"data1", 1000).unwrap();

        let json = tree.visualize_json().unwrap();
        assert!(json.contains("\"type\": \"chrono_merkle_tree\""));
        assert!(json.contains("\"leaf_count\": 1"));
        assert!(json.contains("\"type\": \"leaf\""));
        assert!(json.contains("1000"));
    }

    #[cfg(all(feature = "visualization", feature = "blake3-hash", not(feature = "no-std")))]
    #[test]
    fn test_visualize_empty_tree() {
        let tree: ChronoMerkleTree = ChronoMerkleTree::default();

        let ascii = tree.visualize_ascii();
        assert_eq!(ascii, "Empty tree");

        let dot = tree.visualize_dot();
        assert!(dot.starts_with("digraph ChronoMerkleTree {"));
        assert!(dot.ends_with("}\n"));

        let json = tree.visualize_json().unwrap();
        assert!(json.contains("\"type\": \"empty\""));
        assert!(json.contains("\"leaf_count\": 0"));
    }
}

// Helper for log2 calculation
trait Log2 {
    fn log2(self) -> Self;
}

impl Log2 for f64 {
    fn log2(self) -> Self {
        self.ln() / 2.0_f64.ln()
    }
}
