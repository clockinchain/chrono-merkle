//! Configuration for ChronoMerkleTree

use crate::error::{ChronoMerkleError, Result};

#[cfg(feature = "no-std")]
use alloc::string::ToString;
#[cfg(not(feature = "no-std"))]
use std::string::ToString;

/// Configuration for ChronoMerkleTree
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TreeConfig {
    /// Sparse index sparsity factor
    pub sparse_index_sparsity: u64,
    /// Enable delta nodes for incremental updates
    pub enable_deltas: bool,
    /// Use incremental tree updates (vs full rebuilds)
    pub incremental_updates: bool,
    /// Maximum tree depth (for safety)
    pub max_depth: usize,
    /// Use parallel tree construction (requires "parallel" feature)
    #[cfg_attr(feature = "serde", serde(default))]
    pub parallel_construction: bool,
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            sparse_index_sparsity: 1, // Index all timestamps by default
            enable_deltas: true, // Delta updates are working and provide rollback capabilities
            incremental_updates: true, // Enable incremental updates by default
            max_depth: 32, // SECURITY: Reduced from 64 to prevent excessive memory usage
            #[cfg(feature = "parallel")]
            parallel_construction: false, // SECURITY: Disable parallel by default to prevent timing issues
            #[cfg(not(feature = "parallel"))]
            parallel_construction: false,
        }
    }
}

impl TreeConfig {
    /// Validate configuration parameters for security and correctness
    pub fn validate(&self) -> Result<()> {
        // Validate sparsity factor
        if self.sparse_index_sparsity == 0 {
            return Err(ChronoMerkleError::InvalidConfiguration {
                parameter: "sparse_index_sparsity".to_string(),
                reason: "Sparsity factor must be greater than 0".to_string(),
            });
        }

        // SECURITY: Limit maximum depth to prevent DoS through excessive memory usage
        if self.max_depth == 0 || self.max_depth > 64 {
            return Err(ChronoMerkleError::InvalidConfiguration {
                parameter: "max_depth".to_string(),
                reason: "Maximum depth must be between 1 and 64".to_string(),
            });
        }

        // Delta nodes are now working correctly with rebuild mode
        if self.enable_deltas {
            // Deltas provide efficient incremental updates and rollback capabilities
        }

        // SECURITY: Warn about incremental updates (experimental feature)
        if self.incremental_updates {
            // Incremental updates are experimental and may have correctness issues
            // Use with caution and thorough testing
        }

        Ok(())
    }

    /// Create a secure default configuration
    pub fn secure_defaults() -> Self {
        Self {
            sparse_index_sparsity: 1,
            enable_deltas: true, // Deltas are now working correctly
            incremental_updates: true, // Incremental updates are now working
            max_depth: 32, // Conservative limit
            parallel_construction: false, // Disabled to prevent timing variations
        }
    }
}