//! # ChronoMerkle Tree
//!
//! A time-aware Merkle tree implementation with delta-based updates, programmable nodes,
//! and sparse timestamp indexing. Designed for blockchain and distributed systems.
//!
//! ## Features
//!
//! - **Time-aware**: Each leaf includes a timestamp for efficient time-based queries
//! - **Delta-based updates**: Only affected branches are recomputed on insert
//! - **Programmable nodes**: Custom validation logic at internal nodes
//! - **Sparse indexing**: Fast timestamp-based lookups and range queries
//! - **Generic**: Works with any hash function and hash size
//!
//! ## Example
//!
//! ```rust
//! use chrono_merkle::{ChronoMerkleTree, Blake3Hasher};
//!
//! let mut tree = ChronoMerkleTree::new(Blake3Hasher::default());
//! tree.insert(b"data1", 1000).unwrap();
//! tree.insert(b"data2", 1001).unwrap();
//!
//! let proof = tree.generate_proof(0).unwrap();
//! assert!(tree.verify_proof(&proof).unwrap());
//! ```

#![cfg_attr(feature = "no-std", no_std)]

#[cfg(feature = "no-std")]
extern crate alloc;

pub mod error;
pub mod hash;
pub mod node;
pub mod proof;
pub mod security;
pub mod sparse_index;
pub mod storage;
pub mod tree;
pub mod traits;

#[cfg(feature = "serde")]
pub mod serde_impl;

#[cfg(feature = "clockhash")]
pub mod clockhash;

#[cfg(feature = "clockhash")]
pub use clockhash::ClockHashAdapter;

// Re-exports
pub use error::ChronoMerkleError;
#[cfg(feature = "blake3-hash")]
pub use hash::Blake3Hasher;
pub use hash::HashFunction;
pub use node::{Node, NodeType};
pub use proof::{ChronoProof, ProofStep};
pub use security::{SecurityEvent, SecurityEventType, SecurityLevel, SecurityLogger, NoOpLogger};
#[cfg(feature = "std")]
pub use security::StdErrLogger;
pub use sparse_index::SparseIndex;
#[cfg(all(feature = "storage", feature = "std", not(feature = "no-std")))]
pub use storage::FileStorage;
pub use storage::MemoryStorage;
pub use tree::{ChronoMerkleTree, TreeConfig};

// Conditionally re-export DefaultHasher based on features
#[cfg(any(feature = "sha2-hash", not(any(feature = "sha2-hash", feature = "blake3-hash"))))]
pub use hash::DefaultHasher;
