//! Visualization methods for ChronoMerkleTree

use crate::error::ChronoMerkleError;
use crate::node::{Node, NodeType};
use crate::tree::ChronoMerkleTree;

#[cfg(feature = "no-std")]
use alloc::{format, string::String, vec::Vec};

impl<H, Hasher, Logger> ChronoMerkleTree<H, Hasher, Logger>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: crate::hash::HashFunction<Output = H> + Sync,
    Logger: crate::security::SecurityLogger,
{
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
            NodeType::Leaf { timestamp, .. } => {
                format!("Leaf(ts={}, hash={:?})", timestamp, node.hash())
            }
            NodeType::Internal { timestamp_range, .. } => {
                format!("Internal({}-{}, hash={:?})", timestamp_range.0, timestamp_range.1, node.hash())
            }
            NodeType::Delta { timestamp, .. } => {
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
            NodeType::Leaf { timestamp, .. } => {
                format!("Leaf\\nts={}\\nhash={:?}", timestamp, node.hash())
            }
            NodeType::Internal { timestamp_range, .. } => {
                format!("Internal\\n{}-{}\\nhash={:?}", timestamp_range.0, timestamp_range.1, node.hash())
            }
            NodeType::Delta { timestamp, .. } => {
                format!("Delta\\nts={}\\nhash={:?}", timestamp, node.hash())
            }
        }
    }

    /// Get color for node type
    #[cfg(all(feature = "visualization", not(feature = "no-std")))]
    fn get_node_color(&self, node: &Node<H>) -> &'static str {
        match &node.node_type {
            NodeType::Leaf { .. } => "lightgreen",
            NodeType::Internal { .. } => "lightblue",
            NodeType::Delta { .. } => "lightyellow",
        }
    }

    /// Generate a JSON representation of the tree structure
    #[cfg(all(feature = "visualization", not(feature = "no-std")))]
    pub fn visualize_json(&self) -> crate::error::Result<String> {
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
            NodeType::Leaf { timestamp, hash, data } => {
                json!({
                    "type": "leaf",
                    "timestamp": timestamp,
                    "hash": format!("{:?}", hash),
                    "has_data": data.is_some()
                })
            }
            NodeType::Internal { timestamp_range, hash, left_hash, right_hash } => {
                json!({
                    "type": "internal",
                    "timestamp_range": [timestamp_range.0, timestamp_range.1],
                    "hash": format!("{:?}", hash),
                    "left_hash": format!("{:?}", left_hash),
                    "right_hash": format!("{:?}", right_hash)
                })
            }
            NodeType::Delta { timestamp, delta_hash, base_hash } => {
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