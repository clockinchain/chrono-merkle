//! Tests for ChronoMerkleTree

use crate::node::NodeType;
use crate::tree::ChronoMerkleTree;
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

    let _indices = tree.find_by_timestamp(1000);
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