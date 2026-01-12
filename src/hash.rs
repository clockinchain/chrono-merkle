//! Hash function abstraction for ChronoMerkle Tree

use crate::error::Result;

#[cfg(feature = "no-std")]
use alloc::vec::Vec;

/// Trait for hash functions used in ChronoMerkle trees
pub trait HashFunction {
    /// The output type of the hash function
    type Output: AsRef<[u8]> + Clone + Eq + core::fmt::Debug;

    /// Hash a single piece of data
    fn hash(&self, data: &[u8]) -> Self::Output;

    /// Hash a pair of hashes (for internal nodes)
    fn hash_pair(&self, left: &Self::Output, right: &Self::Output) -> Self::Output {
        let mut combined = Vec::with_capacity(left.as_ref().len() + right.as_ref().len());
        combined.extend_from_slice(left.as_ref());
        combined.extend_from_slice(right.as_ref());
        self.hash(&combined)
    }

    /// Hash multiple pieces of data together
    fn hash_multiple(&self, data: &[&[u8]]) -> Self::Output {
        let mut combined = Vec::new();
        for piece in data {
            combined.extend_from_slice(piece);
        }
        self.hash(&combined)
    }
}

/// Default hasher using SHA-256
#[cfg(feature = "sha2-hash")]
#[derive(Debug, Clone, Default)]
pub struct DefaultHasher;

#[cfg(feature = "sha2-hash")]
impl HashFunction for DefaultHasher {
    type Output = [u8; 32];

    fn hash(&self, data: &[u8]) -> Self::Output {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

/// Blake3 hasher (recommended for performance)
#[cfg(feature = "blake3-hash")]
#[derive(Debug, Clone, Default)]
pub struct Blake3Hasher;

#[cfg(feature = "blake3-hash")]
impl HashFunction for Blake3Hasher {
    type Output = [u8; 32];

    fn hash(&self, data: &[u8]) -> Self::Output {
        *blake3::hash(data).as_bytes()
    }
}

/// Fallback hasher when no hash feature is enabled
/// SECURITY WARNING: This hasher is NOT cryptographically secure and should NEVER be used in production.
/// It exists only to prevent compilation errors, but will panic at runtime to prevent accidental insecure usage.
#[cfg(not(any(feature = "sha2-hash", feature = "blake3-hash")))]
#[derive(Debug, Clone, Default)]
pub struct DefaultHasher;

#[cfg(not(any(feature = "sha2-hash", feature = "blake3-hash")))]
impl HashFunction for DefaultHasher {
    type Output = [u8; 32];

    fn hash(&self, data: &[u8]) -> Self::Output {
        // SECURITY: Panic to prevent accidental use of insecure hasher
        panic!(
            "SECURITY ERROR: No cryptographic hash function enabled! \
             Enable either 'sha2-hash' or 'blake3-hash' feature for secure operation. \
             The fallback hasher is NOT cryptographically secure and should never be used."
        );
    }
}

/// Helper function to convert hash output to bytes
pub fn hash_to_bytes<H: HashFunction>(hasher: &H, data: &[u8]) -> Result<Vec<u8>> {
    Ok(hasher.hash(data).as_ref().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "blake3-hash")]
    #[test]
    fn test_blake3_hasher() {
        let hasher = Blake3Hasher::default();
        let data = b"test data";
        let hash1 = hasher.hash(data);
        let hash2 = hasher.hash(data);
        assert_eq!(hash1, hash2);
        assert_ne!(hasher.hash(b"different"), hash1);
    }

    #[cfg(feature = "sha2-hash")]
    #[test]
    fn test_sha2_hasher() {
        let hasher = DefaultHasher::default();
        let data = b"test data";
        let hash1 = hasher.hash(data);
        let hash2 = hasher.hash(data);
        assert_eq!(hash1, hash2);
        assert_ne!(hasher.hash(b"different"), hash1);
    }

    #[test]
    fn test_hash_pair() {
        #[cfg(feature = "blake3-hash")]
        let hasher = Blake3Hasher::default();
        #[cfg(not(feature = "blake3-hash"))]
        let hasher = DefaultHasher::default();

        let left = hasher.hash(b"left");
        let right = hasher.hash(b"right");
        let pair_hash = hasher.hash_pair(&left, &right);
        
        // Should be deterministic
        let pair_hash2 = hasher.hash_pair(&left, &right);
        assert_eq!(pair_hash, pair_hash2);
        
        // Should be different from individual hashes
        assert_ne!(pair_hash, left);
        assert_ne!(pair_hash, right);
    }
}
