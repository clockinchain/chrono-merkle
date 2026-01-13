//! Node types for ChronoMerkle Tree

use crate::error::{ChronoMerkleError, Result};
#[cfg(feature = "no-std")]
use alloc::{string::ToString, sync::Arc, vec::Vec};

/// Types of nodes in the ChronoMerkle tree
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NodeType<H> {
    /// Leaf node containing data hash and timestamp
    Leaf {
        /// Hash of the leaf data
        hash: H,
        /// Timestamp associated with this leaf
        timestamp: u64,
        /// Optional leaf data (for proof generation)
        data: Option<Vec<u8>>,
    },
    /// Delta node storing incremental changes
    Delta {
        /// Hash of the delta (change)
        delta_hash: H,
        /// Base hash before the delta
        base_hash: H,
        /// Timestamp when delta was applied
        timestamp: u64,
    },
    /// Internal node connecting two subtrees
    Internal {
        /// Computed hash of this internal node (hash_pair(left_hash, right_hash))
        hash: H,
        /// Hash of left child
        left_hash: H,
        /// Hash of right child
        right_hash: H,
        /// Time range covered by this subtree (start, end)
        timestamp_range: (u64, u64),
    },
}

/// A node in the ChronoMerkle tree
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct Node<H> {
    /// The type of this node
    pub node_type: NodeType<H>,
    /// Child nodes (for tree traversal)
    pub children: Vec<Node<H>>,
}

impl<H: Clone> Clone for NodeType<H> {
    fn clone(&self) -> Self {
        match self {
            NodeType::Leaf { hash, timestamp, data } => NodeType::Leaf {
                hash: hash.clone(),
                timestamp: *timestamp,
                data: data.clone(),
            },
            NodeType::Delta { delta_hash, base_hash, timestamp } => NodeType::Delta {
                delta_hash: delta_hash.clone(),
                base_hash: base_hash.clone(),
                timestamp: *timestamp,
            },
            NodeType::Internal { hash, left_hash, right_hash, timestamp_range } => NodeType::Internal {
                hash: hash.clone(),
                left_hash: left_hash.clone(),
                right_hash: right_hash.clone(),
                timestamp_range: *timestamp_range,
            },
        }
    }
}

impl<H: AsRef<[u8]> + Clone> core::fmt::Debug for NodeType<H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NodeType::Leaf { hash, timestamp, data } => {
                f.debug_struct("Leaf")
                    .field("hash", &format_args!("{:?}", hash.as_ref()))
                    .field("timestamp", timestamp)
                    .field("data_len", &data.as_ref().map(|d| d.len()))
                    .finish()
            }
            NodeType::Delta { delta_hash, base_hash, timestamp } => {
                f.debug_struct("Delta")
                    .field("delta_hash", &format_args!("{:?}", delta_hash.as_ref()))
                    .field("base_hash", &format_args!("{:?}", base_hash.as_ref()))
                    .field("timestamp", timestamp)
                    .finish()
            }
            NodeType::Internal { hash, left_hash, right_hash, timestamp_range } => {
                f.debug_struct("Internal")
                    .field("hash", &format_args!("{:?}", hash.as_ref()))
                    .field("left_hash", &format_args!("{:?}", left_hash.as_ref()))
                    .field("right_hash", &format_args!("{:?}", right_hash.as_ref()))
                    .field("timestamp_range", timestamp_range)
                    .finish()
            }
        }
    }
}

impl<H: AsRef<[u8]> + Clone> core::fmt::Debug for Node<H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Node")
            .field("node_type", &self.node_type)
            .field("children", &self.children.len())
            .finish()
    }
}

impl<H: Clone> Node<H> {
    /// Create a new leaf node
    pub fn leaf(hash: H, timestamp: u64, data: Option<Vec<u8>>) -> Self {
        Self {
            node_type: NodeType::Leaf {
                hash,
                timestamp,
                data,
            },
            children: Vec::new(),
        }
    }

    /// Create a new delta node
    pub fn delta(delta_hash: H, base_hash: H, timestamp: u64) -> Self {
        Self {
            node_type: NodeType::Delta {
                delta_hash,
                base_hash,
                timestamp,
            },
            children: Vec::new(),
        }
    }

    /// Create a new internal node
    pub fn internal(hash: H, left_hash: H, right_hash: H, timestamp_range: (u64, u64)) -> Self {
        Self {
            node_type: NodeType::Internal {
                hash,
                left_hash,
                right_hash,
                timestamp_range,
            },
            children: Vec::new(),
        }
    }

    /// Get the hash of this node
    pub fn hash(&self) -> H {
        match &self.node_type {
            NodeType::Leaf { hash, .. } => hash.clone(),
            NodeType::Delta { delta_hash, .. } => delta_hash.clone(),
            NodeType::Internal { hash, .. } => hash.clone(),
        }
    }

    /// Get the timestamp or timestamp range
    pub fn timestamp_info(&self) -> (u64, Option<u64>) {
        match &self.node_type {
            NodeType::Leaf { timestamp, .. } => (*timestamp, None),
            NodeType::Delta { timestamp, .. } => (*timestamp, None),
            NodeType::Internal { timestamp_range, .. } => (timestamp_range.0, Some(timestamp_range.1)),
        }
    }

    /// Check if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        matches!(self.node_type, NodeType::Leaf { .. })
    }

    /// Check if this is a delta node
    pub fn is_delta(&self) -> bool {
        matches!(self.node_type, NodeType::Delta { .. })
    }


    /// Check if this is an internal node
    pub fn is_internal(&self) -> bool {
        matches!(self.node_type, NodeType::Internal { .. })
    }

    /// Validate data using programmable node validator (if applicable)
    /// Note: Programmable nodes are not currently supported with storage
    pub fn validate(&self, _data: &[u8]) -> Result<bool> {
        Err(ChronoMerkleError::InvalidNodeType {
            operation: "validate".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaf_node() {
        let hash = [1u8; 32];
        let node = Node::leaf(hash, 1000, Some(b"data".to_vec()));
        assert!(node.is_leaf());
        assert_eq!(node.hash(), hash);
        assert_eq!(node.timestamp_info(), (1000, None));
    }

    #[test]
    fn test_delta_node() {
        let delta_hash = [2u8; 32];
        let base_hash = [1u8; 32];
        let node = Node::delta(delta_hash, base_hash, 1001);
        assert!(node.is_delta());
        assert_eq!(node.hash(), delta_hash);
    }

    #[test]
    fn test_internal_node() {
        let left = [1u8; 32];
        let right = [2u8; 32];
        // For testing, we'll use a simple hash combination
        let mut internal_hash = [0u8; 32];
        for i in 0..32 {
            internal_hash[i] = left[i].wrapping_add(right[i]);
        }
        let node = Node::internal(internal_hash, left, right, (1000, 2000));
        assert!(node.is_internal());
        assert_eq!(node.hash(), internal_hash);
        assert_eq!(node.timestamp_info(), (1000, Some(2000)));
    }

}
