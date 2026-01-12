//! # Basic Usage Example
//!
//! This example demonstrates the core functionality of the ChronoMerkle tree:
//! - Creating a new tree
//! - Inserting data with timestamps
//! - Generating and verifying proofs
//! - Time-based queries

use chrono_merkle::{ChronoMerkleTree, Blake3Hasher};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🕒 ChronoMerkle Tree - Basic Usage Example\n");

    // Create a new ChronoMerkle tree with Blake3 hasher
    let mut tree = ChronoMerkleTree::new(Blake3Hasher::default());
    println!("✓ Created new ChronoMerkle tree");

    // Insert some data with timestamps
    let data = vec![
        (b"Hello World".as_slice(), 1000),
        (b"ChronoMerkle".as_slice(), 1001),
        (b"Time-aware".as_slice(), 1002),
        (b"Merkle Tree".as_slice(), 1003),
    ];

    println!("📥 Inserting data with timestamps:");
    for (data_item, timestamp) in &data {
        tree.insert(*data_item, *timestamp)?;
        println!("  - {:?} at timestamp {}", String::from_utf8_lossy(data_item), timestamp);
    }

    // Get the current root hash
    if let Some(root) = tree.root() {
        println!("\n🌳 Current tree root: {:x}", root.iter().fold(0u64, |acc, &b| acc.wrapping_mul(256).wrapping_add(b as u64)));
    }

    // Generate a proof for the first leaf
    let proof = tree.generate_proof(0)?;
    println!("\n🔐 Generated proof for leaf at index 0");
    println!("  Proof has {} steps", proof.path.len());

    // Verify the proof
    let is_valid = tree.verify_proof(&proof)?;
    println!("  Proof verification: {}", if is_valid { "✅ VALID" } else { "❌ INVALID" });

    // Demonstrate time-based queries
    println!("\n⏰ Time-based queries:");

    // Find all leaves within a time range
    let range_results = tree.find_range(1001, 1003);
    println!("  Leaves in timestamp range [1001, 1003]: {:?}", range_results);

    // Find exact timestamp match
    let exact_results = tree.find_by_timestamp(1002);
    println!("  Leaves with exact timestamp 1002: {:?}", exact_results);

    // Demonstrate tree statistics
    println!("\n📊 Tree statistics:");
    println!("  Total leaves: {}", tree.leaf_count());
    println!("  Tree depth: {}", tree.depth());

    // Show some timestamp-based queries
    let range_results = tree.find_range(1000, 1002);
    println!("  Leaves in timestamp range [1000, 1002]: {:?}", range_results);

    println!("\n🎉 Basic usage example completed successfully!");

    Ok(())
}