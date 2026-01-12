//! Sparse timestamp-based indexing for ChronoMerkle Tree


#[cfg(feature = "no-std")]
use alloc::{collections::BTreeMap, vec::Vec};
#[cfg(not(feature = "no-std"))]
use std::{collections::BTreeMap, vec::Vec};

/// Sparse index for efficient timestamp-based lookups
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct SparseIndex {
    /// Map from timestamp to leaf index
    index: BTreeMap<u64, usize>,
    /// Store every Nth timestamp (sparsity factor)
    sparsity: u64,
}

impl SparseIndex {
    /// Create a new sparse index with the given sparsity
    ///
    /// # Arguments
    ///
    /// * `sparsity` - Store every Nth timestamp (1 = store all, 10 = store every 10th)
    pub fn new(sparsity: u64) -> Self {
        Self {
            index: BTreeMap::new(),
            sparsity: if sparsity == 0 { 1 } else { sparsity },
        }
    }

    /// Insert a timestamp and leaf index into the index
    pub fn insert(&mut self, timestamp: u64, leaf_index: usize) {
        // Only index if it matches the sparsity pattern
        if timestamp % self.sparsity == 0 {
            self.index.insert(timestamp, leaf_index);
        }
    }

    /// Find all leaf indices within a time range
    ///
    /// # Arguments
    ///
    /// * `start` - Start timestamp (inclusive)
    /// * `end` - End timestamp (inclusive)
    ///
    /// # Returns
    ///
    /// Vector of leaf indices that fall within the range
    pub fn find_range(&self, start: u64, end: u64) -> Vec<usize> {
        self.index
            .range(start..=end)
            .map(|(_, &idx)| idx)
            .collect()
    }

    /// Find the nearest indexed timestamp to the given timestamp
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Target timestamp
    ///
    /// # Returns
    ///
    /// Option containing the leaf index if found
    pub fn find_nearest(&self, timestamp: u64) -> Option<usize> {
        // Find the largest timestamp <= target
        let before = self.index.range(..=timestamp).next_back();
        
        // Find the smallest timestamp >= target
        let after = self.index.range(timestamp..).next();

        match (before, after) {
            (Some((t1, &idx1)), Some((t2, &idx2))) => {
                // Return the closer one, prefer "before" in case of tie
                if timestamp - t1 <= t2 - timestamp {
                    Some(idx1)
                } else {
                    Some(idx2)
                }
            }
            (Some((_, &idx)), None) => Some(idx),
            (None, Some((_, &idx))) => Some(idx),
            (None, None) => None,
        }
    }

    /// Find all timestamps that match a specific timestamp (exact match)
    pub fn find_exact(&self, timestamp: u64) -> Option<usize> {
        self.index.get(&timestamp).copied()
    }

    /// Get the number of indexed entries
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Get the sparsity factor
    pub fn sparsity(&self) -> u64 {
        self.sparsity
    }

    /// Clear all entries from the index
    pub fn clear(&mut self) {
        self.index.clear();
    }

    /// Get all indexed timestamps
    pub fn timestamps(&self) -> Vec<u64> {
        self.index.keys().copied().collect()
    }

    /// Iterate over all (timestamp, leaf_index) pairs
    pub fn iter(&self) -> impl Iterator<Item = (&u64, &usize)> {
        self.index.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_index_insert() {
        let mut index = SparseIndex::new(1); // Store all
        index.insert(1000, 0);
        index.insert(1001, 1);
        index.insert(1002, 2);

        assert_eq!(index.len(), 3);
        assert_eq!(index.find_exact(1000), Some(0));
        assert_eq!(index.find_exact(1001), Some(1));
    }

    #[test]
    fn test_sparse_index_sparsity() {
        let mut index = SparseIndex::new(10); // Store every 10th
        index.insert(1000, 0); // Will be stored (1000 % 10 == 0)
        index.insert(1005, 1); // Won't be stored
        index.insert(1010, 2); // Will be stored

        assert_eq!(index.len(), 2);
        assert_eq!(index.find_exact(1000), Some(0));
        assert_eq!(index.find_exact(1005), None);
        assert_eq!(index.find_exact(1010), Some(2));
    }

    #[test]
    fn test_find_range() {
        let mut index = SparseIndex::new(1);
        for i in 0..10 {
            index.insert(1000 + i as u64, i);
        }

        let results = index.find_range(1002, 1005);
        assert_eq!(results.len(), 4);
        assert_eq!(results, Vec::from([2, 3, 4, 5]));
    }

    #[test]
    fn test_find_nearest() {
        let mut index = SparseIndex::new(1);
        index.insert(1000, 0);
        index.insert(1010, 1);
        index.insert(1020, 2);

        assert_eq!(index.find_nearest(1005), Some(0)); // Equidistant, prefer before
        assert_eq!(index.find_nearest(1015), Some(1)); // Equidistant, prefer before
        assert_eq!(index.find_nearest(1000), Some(0)); // Exact match
    }

    #[test]
    fn test_empty_index() {
        let index = SparseIndex::new(1);
        assert!(index.is_empty());
        assert_eq!(index.find_nearest(1000), None);
        assert_eq!(index.find_range(0, 1000), Vec::<usize>::new());
    }
}
