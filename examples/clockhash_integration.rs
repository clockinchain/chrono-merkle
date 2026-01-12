//! # ClockHash Integration Example
//!
//! This example demonstrates ChronoMerkle's integration with ClockHash
//! for trace compression and time-aware proof generation.
//!
//! ClockHash is a time-aware cryptographic accumulator that compresses
//! execution traces while maintaining time-based verification capabilities.

use chrono_merkle::{Blake3Hasher, DefaultChronoMerkleTree};
#[cfg(feature = "clockhash")]
use chrono_merkle::ClockHashAdapter;

/// Simulated execution trace entry
#[derive(Debug, Clone)]
struct TraceEntry {
    timestamp: u64,
    operation: String,
    data: Vec<u8>,
}

impl TraceEntry {
    fn new(timestamp: u64, operation: &str, data: &[u8]) -> Self {
        Self {
            timestamp,
            operation: operation.to_string(),
            data: data.to_vec(),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(self.operation.as_bytes());
        bytes.extend_from_slice(&self.data);
        bytes
    }
}

/// Simulate a program execution trace
fn generate_execution_trace() -> Vec<TraceEntry> {
    vec![
        TraceEntry::new(1000, "load", b"variable_x"),
        TraceEntry::new(1001, "add", b"1"),
        TraceEntry::new(1002, "store", b"result"),
        TraceEntry::new(1003, "load", b"variable_y"),
        TraceEntry::new(1004, "multiply", b"2"),
        TraceEntry::new(1005, "jump_if", b"condition"),
        TraceEntry::new(1006, "call", b"function_a"),
        TraceEntry::new(1007, "return", b"value"),
        TraceEntry::new(1008, "load", b"array[0]"),
        TraceEntry::new(1009, "add", b"offset"),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("⏰ ChronoMerkle Tree - ClockHash Integration Example\n");

    // Generate a simulated execution trace
    let trace = generate_execution_trace();
    println!("📊 Generated execution trace with {} entries", trace.len());

    #[cfg(feature = "clockhash")]
    {
        // Create ClockHash adapter
        let mut adapter = ClockHashAdapter::new(1000); // Start at time slice 1000
        println!("✓ Created ClockHash adapter for time slice 1000");

        // Add trace entries to the adapter
        println!("\n📥 Adding trace entries to ClockHash adapter:");
        for (i, entry) in trace.iter().enumerate() {
            adapter.add_trace_block(&entry.to_bytes())?;
            println!("  {:2}. {} at t={} -> {}", i + 1, entry.operation, entry.timestamp,
                     if entry.data.is_empty() { "no_data".to_string() } else { format!("{:?}", String::from_utf8_lossy(&entry.data)) });
        }

        // Compute trace root (T_root) for ClockHash verification
        let trace_root = adapter.compute_trace_root()?;
        println!("\n🌳 Computed trace root (T_root):");
        println!("   {:x}", trace_root.iter().fold(0u64, |acc: u64, &b| acc.wrapping_mul(256).wrapping_add(b as u64)));

        // Demonstrate time-based queries
        println!("\n⏰ Time-based trace queries:");

        let query_times = vec![1000, 1002, 1005, 1010]; // Some timestamps to query

        for &query_time in &query_times {
            let results = adapter.query_by_time_slice(query_time);
            println!("  Time slice {}: {} trace entries", query_time, results.len());
            for &idx in &results {
                if idx < trace.len() {
                    let entry = &trace[idx];
                    println!("    └─ {} at t={}", entry.operation, entry.timestamp);
                }
            }
        }
    }

    #[cfg(not(feature = "clockhash"))]
    {
        println!("⚠️  ClockHash feature not enabled - skipping ClockHash adapter demo");
        println!("   To enable ClockHash features, run with: cargo run --example clockhash_integration --features clockhash");
    }

    // Demonstrate integration with regular ChronoMerkle tree for comparison
    println!("\n🔄 Comparison with regular ChronoMerkle tree:");

    let mut regular_tree = DefaultChronoMerkleTree::new(Blake3Hasher::default());
    for entry in &trace {
        regular_tree.insert(&entry.to_bytes(), entry.timestamp)?;
    }

    let regular_root = regular_tree.root().unwrap();
    println!("  Regular tree root: {:x}", regular_root.iter().fold(0u64, |acc: u64, &b| acc.wrapping_mul(256).wrapping_add(b as u64)));

    #[cfg(feature = "clockhash")]
    {
        // Create ClockHash adapter for comparison
        let mut adapter = ClockHashAdapter::new(1000);
        for entry in &trace {
            adapter.add_trace_block(&entry.to_bytes())?;
        }
        let trace_root = adapter.compute_trace_root()?;
        println!("  ClockHash T_root:  {:x}", trace_root.iter().fold(0u64, |acc: u64, &b| acc.wrapping_mul(256).wrapping_add(b as u64)));
        println!("  Roots match: {}", regular_root == trace_root);
    }

    #[cfg(not(feature = "clockhash"))]
    {
        println!("  ClockHash T_root:  (feature not enabled)");
        println!("  Roots match: N/A");
    }

    // Demonstrate proof generation capabilities
    if trace.len() > 0 {
        println!("\n🔐 Proof generation capabilities:");

        // Generate proof for first trace entry
        let proof = regular_tree.generate_proof(0)?;
        let is_valid = regular_tree.verify_proof(&proof)?;
        println!("  Proof for trace entry 0: {}", if is_valid { "✅ VALID" } else { "❌ INVALID" });

        // Time range queries
        let time_range_results = regular_tree.find_range(1002, 1006);
        println!("  Trace entries in time range [1002, 1006]: {:?}", time_range_results);

        let exact_time_results = regular_tree.find_by_timestamp(1004);
        println!("  Trace entries at exact time 1004: {:?}", exact_time_results);
    }

    // Demonstrate compression benefits (conceptual)
    println!("\n🗜️  Compression Benefits:");

    #[cfg(feature = "clockhash")]
    {
        let mut adapter = ClockHashAdapter::new(1000);
        for entry in &trace {
            adapter.add_trace_block(&entry.to_bytes())?;
        }

        println!("  Original trace: {} entries", trace.len());
        println!("  Sparse index size: {} timestamps", adapter.timestamps().len());
        println!("  Compression ratio: {:.2}x", trace.len() as f64 / adapter.timestamps().len() as f64);

        // Show indexed timestamps
        let indexed_times = adapter.timestamps();
        println!("  Indexed timestamps: {:?}", indexed_times);
    }

    #[cfg(not(feature = "clockhash"))]
    {
        println!("  ClockHash compression demo requires --features clockhash");
        println!("  Regular tree has {} leaves", regular_tree.leaf_count());
    }

    println!("\n🎉 ClockHash integration example completed!");
    println!("💡 ClockHash enables efficient time-aware trace compression with Merkle tree verification");

    Ok(())
}