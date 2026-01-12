//! Tree rebuilding logic for ChronoMerkleTree

use crate::error::Result;
use crate::hash::HashFunction;
use crate::node::Node;
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
    /// Rebuild the entire tree from leaves (used when incremental updates are disabled)
    pub(crate) fn rebuild_tree(&mut self) -> Result<()> {
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
            .filter(|node| matches!(node.node_type, crate::node::NodeType::Leaf { .. }))
            .cloned()
            .collect();

        // Clear all nodes and re-insert the leaves
        self.nodes.clear();
        self.nodes.extend(current_leaves);

        // Build the complete binary tree by iteratively combining nodes
        let mut current_start = 0;
        let mut current_count = self.leaf_count;

        while current_count > 1 {
            let next_count = current_count.div_ceil(2);

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
    pub(crate) fn rebuild_tree_parallel(&mut self) -> Result<()> {
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
            .filter(|node| matches!(node.node_type, crate::node::NodeType::Leaf { .. }))
            .cloned()
            .collect();

        // Clear all nodes and re-insert the leaves
        self.nodes.clear();
        self.nodes.extend(current_leaves);

        // Build the complete binary tree by iteratively combining nodes
        let mut current_start = 0;
        let mut current_count = self.leaf_count;

        while current_count > 1 {
            let next_count = current_count.div_ceil(2);

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
    pub(crate) fn update_tree_incremental(&mut self) -> Result<()> {
        // For now, incremental updates fall back to full rebuild
        // TODO: Implement true incremental updates
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

        Ok(())
    }

    /// Rebuild internal nodes starting from a leaf up to the root
    #[allow(dead_code)]
    pub(crate) fn rebuild_from_leaf(&mut self, leaf_index: usize) -> Result<()> {
        // Rebuild internal nodes starting from the affected leaf up to the root
        // This is more efficient than rebuilding the entire tree

        if self.leaf_count <= 1 {
            return Ok(());
        }


        // Leaves are stored at the beginning of the nodes array (indices 0, 1, 2, ...)
        let leaf_node_index = leaf_index;

        // Start from this leaf and work up to the root
        let mut current_index = leaf_node_index;

        while current_index > 0 {
            // Find the parent of current_index
            let parent_index = (current_index - 1) / 2;

            // Get both children of this parent
            let left_child_index = 2 * parent_index + 1;
            let right_child_index = 2 * parent_index + 2;

            // Make sure both children exist
            if left_child_index >= self.nodes.len() || right_child_index >= self.nodes.len() {
                // If we don't have both children yet, we can't rebuild this parent
                break;
            }

            let left_child = &self.nodes[left_child_index];
            let right_child = &self.nodes[right_child_index];

            // Compute new parent hash
            let parent_hash = self.hasher.hash_pair(&left_child.hash(), &right_child.hash());

            // Compute timestamp range
            let left_ts = left_child.timestamp_info();
            let right_ts = right_child.timestamp_info();
            let start = left_ts.0.min(right_ts.0);
            let end = match (left_ts.1, right_ts.1) {
                (Some(l), Some(r)) => Some(l.max(r)),
                (Some(l), None) => Some(l.max(right_ts.0)),
                (None, Some(r)) => Some(r.max(left_ts.0)),
                (None, None) => Some(left_ts.0.max(right_ts.0)),
            };

            // Update the parent node
            let new_parent = Node::internal(
                parent_hash,
                left_child.hash(),
                right_child.hash(),
                (start, end.unwrap_or(start))
            );

            self.nodes[parent_index] = new_parent;

            // Move up to parent
            current_index = parent_index;
        }

        Ok(())
    }

    /// Ensure the tree has enough capacity for all nodes
    pub(crate) fn ensure_tree_capacity(&mut self) -> Result<()> {
        // Calculate the total number of nodes needed for a complete binary tree
        // with self.leaf_count leaves

        if self.leaf_count == 0 {
            return Ok(());
        }

        let total_nodes_needed = if self.leaf_count == 1 {
            1
        } else {
            // Find the smallest power of 2 greater than or equal to leaf_count
            let mut capacity = 1;
            while capacity < self.leaf_count {
                capacity *= 2;
            }
            // Total nodes = 2^ceil(log2(leaf_count)) - 1
            capacity * 2 - 1
        };

        // Ensure we have enough space by adding placeholder nodes
        while self.nodes.len() < total_nodes_needed {
            // Add placeholder internal nodes that will be properly initialized
            let zero_hash = self.hasher.hash(b""); // Use hash of empty string as placeholder
            let placeholder = Node::internal(
                zero_hash.clone(), // Will be updated
                zero_hash.clone(), // Will be updated
                zero_hash, // Will be updated
                (0, 0) // Will be updated
            );
            self.nodes.push(placeholder);
        }

        Ok(())
    }
}