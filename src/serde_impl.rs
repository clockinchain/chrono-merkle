//! Serialization support for ChronoMerkle Tree

// Re-export serde traits for convenience when the feature is enabled
#[cfg(feature = "serde")]
pub use serde::{Deserialize, Serialize};

// Note: ChronoMerkleTree itself is not directly serializable due to containing
// a HashFunction instance. For serialization, extract the leaves and reconstruct
// the tree. See the examples for usage patterns.
//
// All core structures support serde:
// - Node<H> - Individual tree nodes
// - NodeType<H> - Node type variants
// - ChronoProof<H> - Merkle proofs
// - ProofStep<H> - Proof path steps
// - SparseIndex - Timestamp index
// - TreeConfig - Tree configuration
//
// For full tree serialization including deltas, use:
// - extract_leaves_and_deltas() -> (Vec<Node<H>>, Vec<Node<H>>)
// - reconstruct_from_leaves_and_deltas()