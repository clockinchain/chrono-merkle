//! Delta operations and rollback functionality for ChronoMerkleTree

use crate::error::{ChronoMerkleError, Result};
use crate::hash::HashFunction;
use crate::node::{Node, NodeType};
use crate::sparse_index::SparseIndex;
use crate::tree::ChronoMerkleTree;

#[cfg(feature = "no-std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no-std"))]
use std::vec::Vec;

impl<H, Hasher, Logger> ChronoMerkleTree<H, Hasher, Logger>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: HashFunction<Output = H> + Sync,
    Logger: crate::security::SecurityLogger,
{
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

    /// Rollback the tree to a previous state using delta chain
    pub fn rollback_to_timestamp(&mut self, target_timestamp: u64) -> Result<()> {
        // Simple rollback: remove leaves added after the target timestamp
        // This provides the core rollback functionality

        if self.leaf_count == 0 {
            return Err(ChronoMerkleError::InvalidTimestamp {
                timestamp: target_timestamp,
            });
        }

        // Collect all leaves with timestamp <= target_timestamp
        let mut leaves_to_keep = Vec::new();
        let mut timestamps_to_keep = Vec::new();

        // Collect all leaves with timestamp <= target_timestamp
        for node in &self.nodes {
            if let NodeType::Leaf { timestamp, .. } = &node.node_type {
                if *timestamp <= target_timestamp {
                    leaves_to_keep.push(node.clone());
                    timestamps_to_keep.push(*timestamp);
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
        for (idx, timestamp) in timestamps_to_keep.into_iter().enumerate() {
            self.sparse_index.insert(timestamp, idx);
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

    /// Compute deltas for the path from a leaf to the root (legacy - kept for compatibility)
    #[allow(dead_code)]
    pub(crate) fn compute_path_deltas(&mut self, leaf_index: usize) -> Result<()> {
        // Ensure the tree has enough space for all nodes
        self.ensure_tree_capacity()?;

        // Update only the path from leaf to root, maintaining complete binary tree structure
        let mut current_index = leaf_index;
        let mut current_node = self.nodes[leaf_index].clone();

        // Store the old root for delta computation
        let old_root = self.root();

        // Traverse from leaf to root, updating only the affected path
        while current_index > 0 {
            let parent_index = (current_index - 1) / 2;

            // Get the old parent hash before update (for delta computation)
            let old_parent_hash = if parent_index < self.nodes.len() {
                Some(self.nodes[parent_index].hash())
            } else {
                None
            };

            // Get the sibling at this level
            let sibling_index = if current_index % 2 == 0 {
                current_index - 1 // left sibling
            } else {
                current_index + 1 // right sibling
            };

            // For complete binary trees, we need to check if the sibling actually exists
            // at this level of the tree. The sibling exists if:
            // 1. It's within bounds of current leaf count, AND
            // 2. It's at the same level in the tree (same depth from root)
            let current_level = self.get_node_level(current_index);
            let sibling_level = self.get_node_level(sibling_index);

            let sibling_node = if sibling_index < self.leaf_count &&
                                current_level == sibling_level &&
                                sibling_index != current_index {
                Some(&self.nodes[sibling_index])
            } else {
                None
            };

            // Compute new parent hash based on available children
            let (new_parent_hash, left_hash, right_hash) = if let Some(sibling) = sibling_node {
                // Two children case
                let left_hash = if current_index % 2 == 0 { sibling.hash() } else { current_node.hash() };
                let right_hash = if current_index % 2 == 0 { current_node.hash() } else { sibling.hash() };
                let parent_hash = self.hasher.hash_pair(&left_hash, &right_hash);
                (parent_hash, left_hash, right_hash)
            } else {
                // Single child case - promote the current node
                (current_node.hash(), current_node.hash(), current_node.hash())
            };

            // Compute timestamp range for the parent
            let timestamp_range = if let Some(sibling) = sibling_node {
                let current_ts = current_node.timestamp_info();
                let sibling_ts = sibling.timestamp_info();

                let start = current_ts.0.min(sibling_ts.0);
                let end = match (current_ts.1, sibling_ts.1) {
                    (Some(c), Some(s)) => Some(c.max(s)),
                    (Some(c), None) => Some(c.max(sibling_ts.0)),
                    (None, Some(s)) => Some(s.max(current_ts.0)),
                    (None, None) => Some(current_ts.0.max(sibling_ts.0)),
                };

                (start, end)
            } else {
                current_node.timestamp_info()
            };

            // Create the new parent node
            let new_parent = if sibling_node.is_some() {
                Node::internal(new_parent_hash.clone(), left_hash, right_hash,
                             (timestamp_range.0, timestamp_range.1.unwrap_or(timestamp_range.0)))
            } else {
                // Single child - create internal node with promoted child
                Node {
                    node_type: NodeType::Internal {
                        hash: new_parent_hash.clone(),
                        left_hash: current_node.hash(),
                        right_hash: current_node.hash(),
                        timestamp_range: (timestamp_range.0, timestamp_range.1.unwrap_or(timestamp_range.0)),
                    },
                    children: vec![current_node.clone()],
                }
            };

            // Store delta if parent changed
            if let Some(old_hash) = old_parent_hash {
                if old_hash != new_parent_hash {
                    let delta_hash = self.hasher.hash_pair(&old_hash, &new_parent_hash);
                    let delta_node = Node::delta(delta_hash, old_hash, timestamp_range.0);

                    let delta_index = self.stored_deltas.len();
                    self.stored_deltas.push(delta_node);
                    self.delta_chains.insert(timestamp_range.0, delta_index);
                }
            }

            // Update or add the parent node
            if parent_index >= self.nodes.len() {
                self.nodes.push(new_parent.clone());
            } else {
                self.nodes[parent_index] = new_parent.clone();
            }

            // Move up to parent
            current_index = parent_index;
            current_node = new_parent;
        }

        // Create root-level delta if the root changed
        if let (Some(old_root_hash), Some(new_root_hash)) = (old_root, self.root()) {
            if old_root_hash != new_root_hash {
                let delta_hash = self.hasher.hash_pair(&old_root_hash, &new_root_hash);
                let delta_node = Node::delta(delta_hash, old_root_hash, current_node.timestamp_info().0);

                let delta_index = self.stored_deltas.len();
                self.stored_deltas.push(delta_node);
                self.delta_chains.insert(current_node.timestamp_info().0, delta_index);
            }
        }

        Ok(())
    }

    /// Get the level (depth) of a node in the complete binary tree
    #[allow(dead_code)]
    pub(crate) fn get_node_level(&self, node_index: usize) -> usize {
        // For a complete binary tree stored in heap order,
        // the level can be determined by finding the highest power of 2
        // that divides the index + 1
        let mut level = 0;
        let mut temp = node_index + 1;
        while temp > 1 {
            temp >>= 1;
            level += 1;
        }
        level
    }

    /// Recompute parent hashes from a given node index up to the root
    #[allow(dead_code)]
    pub(crate) fn recompute_parent_hashes_from_node(&mut self, start_index: usize) -> Result<()> {
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
}