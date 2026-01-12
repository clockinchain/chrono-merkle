//! Core tree operations for ChronoMerkleTree

use crate::error::{ChronoMerkleError, Result};
use crate::hash::HashFunction;
use crate::node::{Node, NodeType};
use crate::security::SecurityLogger;
use crate::tree::ChronoMerkleTree;

#[cfg(feature = "no-std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no-std"))]
use std::vec::Vec;

impl<H, Hasher, Logger> ChronoMerkleTree<H, Hasher, Logger>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: HashFunction<Output = H> + Sync,
    Logger: SecurityLogger,
{
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

    /// Get the timestamp of a leaf by index
    pub fn get_leaf_timestamp(&self, index: usize) -> Result<u64> {
        let (timestamp, _) = self.get_leaf(index)?.timestamp_info();
        Ok(timestamp)
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
        config: crate::config::TreeConfig,
    ) -> Result<Self>
    where
        Logger: Default,
        Hasher: Default,
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
    #[allow(dead_code)]
    pub(crate) fn next_power_of_two(&self, n: usize) -> usize {
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
        self.delta_chains = crate::sparse_index::SparseIndex::new(self.config.sparse_index_sparsity);
        for (i, delta) in self.stored_deltas.iter().enumerate() {
            if let NodeType::Delta { timestamp, .. } = &delta.node_type {
                self.delta_chains.insert(*timestamp, i);
            }
        }
    }
}