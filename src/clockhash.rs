//! ClockHash integration helpers for ChronoMerkle Tree
//!
//! This module provides adapters and utilities for integrating
//! ChronoMerkle with ClockHash's trace compression system.

#[cfg(not(feature = "clockhash"))]
compile_error!("clockhash feature must be enabled to use clockhash module");

#[cfg(feature = "no-std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no-std"))]
use std::vec::Vec;

use crate::{ChronoMerkleTree, Blake3Hasher};
use crate::error::{Result, ChronoMerkleError};

/// Adapter for using ChronoMerkle with ClockHash trace compression
pub struct ClockHashAdapter {
    tree: ChronoMerkleTree<[u8; 32], Blake3Hasher>,
    time_slice: u64,
}

impl ClockHashAdapter {
    /// Create a new adapter for a specific time slice
    pub fn new(time_slice: u64) -> Self {
        Self {
            tree: ChronoMerkleTree::new(Blake3Hasher::default()),
            time_slice,
        }
    }

    /// Add a trace block to the tree
    pub fn add_trace_block(&mut self, block_data: &[u8]) -> Result<()> {
        self.tree.insert(block_data, self.time_slice)
    }

    /// Compute the trace root (T_root) for ClockHash
    pub fn compute_trace_root(&self) -> Result<[u8; 32]> {
        self.tree.root().ok_or(ChronoMerkleError::EmptyTree)
    }

    /// Query trace blocks by time slice
    pub fn query_by_time_slice(&self, time_slice: u64) -> Vec<usize> {
        if time_slice == self.time_slice {
            self.tree.find_by_timestamp(time_slice)
        } else {
            Vec::new()
        }
    }

    /// Query trace blocks in a time range
    pub fn query_time_range(&self, start: u64, end: u64) -> Vec<usize> {
        self.tree.find_range(start, end)
    }

    /// Generate a proof for a trace block
    pub fn generate_proof(&self, block_index: usize) -> Result<crate::ChronoProof<[u8; 32]>> {
        self.tree.generate_proof(block_index)
    }

    /// Get the current time slice
    pub fn time_slice(&self) -> u64 {
        self.time_slice
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clockhash_adapter() {
        let mut adapter = ClockHashAdapter::new(12345);
        adapter.add_trace_block(b"block1").unwrap();
        adapter.add_trace_block(b"block2").unwrap();

        let root = adapter.compute_trace_root().unwrap();
        assert_ne!(root, [0u8; 32]);

        let blocks = adapter.query_by_time_slice(12345);
        assert_eq!(blocks.len(), 2);
    }
}
