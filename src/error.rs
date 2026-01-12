//! Error types for ChronoMerkle Tree

use thiserror::Error;

#[cfg(feature = "no-std")]
use alloc::string::String;

/// Errors that can occur when working with ChronoMerkle trees
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ChronoMerkleError {
    /// Leaf index is out of bounds
    #[error("Leaf index {index} is out of bounds (tree has {leaf_count} leaves)")]
    IndexOutOfBounds { index: usize, leaf_count: usize },

    /// Invalid proof structure
    #[error("Invalid proof structure: {message}")]
    InvalidProof { message: String },

    /// Proof verification failed
    #[error("Proof verification failed: {reason}")]
    ProofVerificationFailed { reason: String },

    /// Time slice mismatch
    #[error("Time slice mismatch: expected {expected}, got {actual}")]
    TimeSliceMismatch { expected: u64, actual: u64 },

    /// Invalid timestamp
    #[error("Invalid timestamp: {timestamp}")]
    InvalidTimestamp { timestamp: u64 },

    /// Hash computation error
    #[error("Hash computation error: {message}")]
    HashError { message: String },

    /// Tree is empty
    #[error("Tree is empty")]
    EmptyTree,

    /// Invalid node type for operation
    #[error("Invalid node type for operation: {operation}")]
    InvalidNodeType { operation: String },

    /// Delta proof generation failed
    #[error("Delta proof generation failed: {reason}")]
    DeltaProofFailed { reason: String },

    /// Programmable node validation failed
    #[error("Programmable node validation failed: {reason}")]
    ValidationFailed { reason: String },

    /// ClockHash integration error
    #[cfg(feature = "clockhash")]
    #[error("ClockHash integration error: {0}")]
    ClockHashError(String),

    /// Serialization error
    #[cfg(any(feature = "serde", feature = "storage"))]
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Deserialization error
    #[cfg(feature = "serde")]
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Storage backend error
    #[cfg(feature = "storage")]
    #[error("Storage error: {reason}")]
    StorageError { reason: String },

    /// Invalid configuration parameter
    #[error("Invalid configuration: {parameter} - {reason}")]
    InvalidConfiguration { parameter: String, reason: String },
}

/// Result type alias for ChronoMerkle operations
pub type Result<T> = core::result::Result<T, ChronoMerkleError>;
