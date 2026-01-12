# ChronoMerkle

[![Crates.io](https://img.shields.io/crates/v/chrono-merkle.svg)](https://crates.io/crates/chrono-merkle)
[![Documentation](https://docs.rs/chrono-merkle/badge.svg)](https://docs.rs/chrono-merkle)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/clockinchain/chrono-merkle)

A time-aware Merkle tree implementation with delta-based updates, programmable nodes, and sparse timestamp indexing. Designed for blockchain applications, audit trails, and time-series data verification.

## ✨ Features

- **⏰ Time-aware**: Each leaf includes a timestamp for efficient time-based queries
- **🔄 Delta-based updates**: Only affected branches are recomputed on insert
- **🎛️ Programmable nodes**: Custom validation logic at internal tree nodes
- **📊 Sparse indexing**: Fast timestamp-based lookups and range queries
- **🔧 Generic**: Works with any hash function and hash size
- **🔐 Proof generation**: Cryptographic proofs with delta chains
- **⏪ Rollback support**: Time-based rollback capabilities
- **💾 Storage backends**: Pluggable storage with file/memory/postgres/redis support
- **🚀 Performance**: Parallel operations and optimized for large datasets

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
chrono-merkle = "0.1"
```

### Feature Flags

- `default`: `serde`, `std`, `blake3-hash`
- `serde`: Serialization support with `serde`
- `std`: Standard library support (enabled by default)
- `blake3-hash`: Blake3 hasher (enabled by default)
- `sha2-hash`: SHA-256 hasher
- `clockhash`: ClockHash integration for trace compression
- `parallel`: Parallel tree operations with Rayon
- `storage`: Storage backend support
- `file-storage`: File-based storage
- `memory-storage`: In-memory storage (enabled with `storage`)
- `postgres-storage`: PostgreSQL storage backend
- `redis-storage`: Redis storage backend
- `no-std`: Embedded/no-std support
- `wasm`: WebAssembly support
- `visualization`: ASCII/DOT/JSON visualization

## 🚀 Quick Start

```rust
use chrono_merkle::{ChronoMerkleTree, Blake3Hasher};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new ChronoMerkle tree
    let mut tree = ChronoMerkleTree::new(Blake3Hasher::default());

    // Insert data with timestamps
    tree.insert(b"Hello World", 1000)?;
    tree.insert(b"ChronoMerkle", 1001)?;
    tree.insert(b"Time-aware", 1002)?;

    // Generate a proof for verification
    let proof = tree.generate_proof(0)?;
    let is_valid = tree.verify_proof(&proof)?;

    assert!(is_valid);
    println!("✅ Proof verified successfully!");

    // Time-based queries
    let results = tree.find_range(1000, 1002);
    println!("Found {} entries in range", results.len());

    Ok(())
}
```

## 📚 Examples

### Basic Usage
```bash
cargo run --example basic_usage
```
Demonstrates core tree operations, proof generation, and time-based queries.

### Blockchain Integration
```bash
cargo run --example blockchain_example
```
Shows how to use ChronoMerkle for blockchain-like operations with transaction validation.

### Programmable Nodes
```bash
cargo run --example programmable_nodes
```
Illustrates custom validation logic at tree nodes for business rules and security policies.

### ClockHash Integration
```bash
cargo run --example clockhash_integration --features clockhash
```
Demonstrates trace compression using ClockHash integration.

## 🏗️ Architecture

### Core Components

- **`ChronoMerkleTree`**: Main tree structure with time-aware operations
- **`SparseIndex`**: Efficient timestamp-based indexing and queries
- **`ChronoProof`**: Cryptographic proofs with delta chain support
- **Node Types**: Leaf, Internal, Delta, and Programmable nodes
- **Storage Backends**: Pluggable persistence layers

### Key Concepts

#### Delta-Based Updates
Instead of rebuilding the entire tree on each insert, ChronoMerkle uses delta nodes to represent changes, significantly improving performance for frequent updates.

#### Sparse Timestamp Indexing
Maintains an index of timestamp-to-leaf mappings with configurable sparsity, enabling fast range queries without scanning all leaves.

#### Programmable Nodes
Internal nodes can contain custom validation functions, allowing business logic, access control, or compliance rules to be enforced at the tree level.

## 🔧 Advanced Usage

### Custom Hash Functions

```rust
use chrono_merkle::{ChronoMerkleTree, HashFunction};

// Implement your own hasher
#[derive(Clone, Default)]
struct MyHasher;

impl HashFunction for MyHasher {
    type Output = [u8; 32];

    fn hash(&self, data: &[u8]) -> Self::Output {
        // Your hashing logic here
        [0; 32] // Placeholder
    }

    fn hash_pair(&self, left: &Self::Output, right: &Self::Output) -> Self::Output {
        // Hash combination logic
        [0; 32] // Placeholder
    }
}

let tree = ChronoMerkleTree::new(MyHasher::default());
```

### Programmable Validation

```rust
use chrono_merkle::{ChronoMerkleTree, Blake3Hasher};

let mut tree = ChronoMerkleTree::new(Blake3Hasher::default());

// Add data that should pass your validation rules
tree.insert(b"valid_data", 1000)?;

// Tree maintains validation state
assert!(tree.is_valid());
```

### Storage Persistence

```rust
use chrono_merkle::{ChronoMerkleTree, Blake3Hasher, MemoryStorage};

#[cfg(feature = "storage")]
{
    let mut tree = ChronoMerkleTree::new(Blake3Hasher::default());
    tree.insert(b"persistent_data", 1000)?;

    // Save to storage
    let storage = MemoryStorage::new();
    tree.save_state(&storage, "my_tree")?;

    // Load from storage
    let loaded_tree = ChronoMerkleTree::load_state(&storage, "my_tree", Blake3Hasher::default())?;
}
```

## 📊 Performance

ChronoMerkle is optimized for:

- **High-frequency updates**: Delta-based approach minimizes recomputation
- **Time-based queries**: Sparse indexing enables O(log n) range queries
- **Large datasets**: Efficient memory usage with configurable sparsity
- **Concurrent access**: Thread-safe operations with `Send + Sync` bounds

### Benchmarks

Run benchmarks with:
```bash
cargo bench
```

## 🤝 Contributing

We welcome contributions! Please see our [contributing guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/clockinchain/chrono-merkle.git
cd chrono-merkle

# Run tests
cargo test

# Run examples
cargo run --example basic_usage

# Run benchmarks
cargo bench

# Generate documentation
cargo doc --open
```

### Code Style

This project uses:
- `rustfmt` for code formatting
- `clippy` for linting
- Comprehensive test coverage

```bash
# Format code
cargo fmt

# Run lints
cargo clippy

# Run tests with coverage
cargo test -- --nocapture
```

## 📄 License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🔗 Related Projects

- [ClockInChain](https://github.com/clockinchain) - Main project using ChronoMerkle
- [ClockHash](https://github.com/clockinchain/clockhash) - Trace compression system
- [Merkle Trees](https://en.wikipedia.org/wiki/Merkle_tree) - Underlying data structure

## 📞 Support

- 📖 [Documentation](https://docs.rs/chrono-merkle)
- 🐛 [Issue Tracker](https://github.com/clockinchain/chrono-merkle/issues)
- 💬 [Discussions](https://github.com/clockinchain/chrono-merkle/discussions)

---

**ChronoMerkle** - Time-aware cryptographic data structures for the next generation of blockchain and distributed systems.