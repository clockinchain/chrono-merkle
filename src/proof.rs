//! Proof generation and verification for ChronoMerkle Tree

use crate::error::{ChronoMerkleError, Result};
use crate::hash::HashFunction;
use crate::security::constant_time_eq;

#[cfg(feature = "no-std")]
use alloc::{string::ToString, vec::Vec};
#[cfg(not(feature = "no-std"))]
use std::vec::Vec;

/// A step in a Merkle proof path
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofStep<H> {
    /// Sibling is on the left (current node is right child)
    Left(H),
    /// Sibling is on the right (current node is left child)
    Right(H),
    /// Delta update step (old hash, new hash)
    Delta(H, H),
}

/// A complete Merkle proof for a leaf
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ChronoProof<H> {
    /// Index of the leaf being proven
    pub leaf_index: usize,
    /// Path from leaf to root (sibling hashes)
    pub path: Vec<ProofStep<H>>,
    /// Optional delta chain for incremental updates
    pub delta_chain: Option<Vec<H>>,
    /// Results from programmable node validations along the path
    pub programmable_results: Vec<bool>,
    /// Timestamp of the leaf
    pub timestamp: u64,
}

impl<H: Clone> ChronoProof<H> {
    /// Create a new proof
    pub fn new(leaf_index: usize, timestamp: u64) -> Self {
        Self {
            leaf_index,
            path: Vec::new(),
            delta_chain: None,
            programmable_results: Vec::new(),
            timestamp,
        }
    }

    /// Add a proof step
    pub fn add_step(&mut self, step: ProofStep<H>) {
        self.path.push(step);
    }

    /// Add a delta to the delta chain
    pub fn add_delta(&mut self, delta: H) {
        if self.delta_chain.is_none() {
            self.delta_chain = Some(Vec::new());
        }
        self.delta_chain.as_mut().unwrap().push(delta);
    }

    /// Add a programmable node validation result
    pub fn add_validation_result(&mut self, result: bool) {
        self.programmable_results.push(result);
    }
}


/// Verify a proof against a root hash
pub fn verify_proof<H, Hasher>(
    proof: &ChronoProof<H>,
    leaf_hash: &H,
    root_hash: &H,
    hasher: &Hasher,
) -> Result<bool>
where
    H: AsRef<[u8]> + Clone + Eq,
    Hasher: HashFunction<Output = H>,
{
    let mut current_hash = leaf_hash.clone();
    let mut delta_index = 0;

    // Verify each step in the path
    for step in &proof.path {
        match step {
            ProofStep::Left(sibling) => {
                // Current is right child, sibling is left
                current_hash = hasher.hash_pair(sibling, &current_hash);
            }
            ProofStep::Right(sibling) => {
                // Current is left child, sibling is right
                current_hash = hasher.hash_pair(&current_hash, sibling);
            }
            ProofStep::Delta(old_hash, new_hash) => {
                // Verify cryptographic delta computation using delta chain
                if constant_time_eq(current_hash.as_ref(), old_hash.as_ref()) {
                    // SECURITY: Ensure new_hash is different from old_hash to prevent no-op deltas
                    if constant_time_eq(new_hash.as_ref(), old_hash.as_ref()) {
                        return Err(ChronoMerkleError::ProofVerificationFailed {
                            reason: "Delta proof cannot have identical old and new hashes".to_string(),
                        });
                    }

                    // Verify delta using the delta chain: delta_hash should equal hash(old_hash, new_hash)
                    if let Some(delta_chain) = &proof.delta_chain {
                        if delta_index >= delta_chain.len() {
                            return Err(ChronoMerkleError::ProofVerificationFailed {
                                reason: "Delta proof missing delta chain entry".to_string(),
                            });
                        }

                        let delta_hash = &delta_chain[delta_index];
                        let expected_delta_hash = hasher.hash_pair(old_hash, new_hash);

                        if !crate::security::constant_time_eq(delta_hash.as_ref(), expected_delta_hash.as_ref()) {
                            return Err(ChronoMerkleError::ProofVerificationFailed {
                                reason: "Delta computation verification failed - invalid delta hash".to_string(),
                            });
                        }

                        delta_index += 1;
                    } else {
                        return Err(ChronoMerkleError::ProofVerificationFailed {
                            reason: "Delta proof requires delta chain but none provided".to_string(),
                        });
                    }

                    current_hash = new_hash.clone();
                } else {
                    return Err(ChronoMerkleError::ProofVerificationFailed {
                        reason: "Delta proof old_hash mismatch with current verification state".to_string(),
                    });
                }
            }
        }
    }

    // Verify programmable node results (all should be true)
    for &result in &proof.programmable_results {
        if !result {
            return Err(ChronoMerkleError::ValidationFailed {
                reason: "Programmable node validation failed".to_string(),
            });
        }
    }

    // SECURITY: Use constant-time comparison to prevent timing attacks
    // Final hash should match root
    Ok(crate::security::constant_time_eq(current_hash.as_ref(), root_hash.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "blake3-hash")]
    #[test]
    fn test_proof_creation() {
        let proof: ChronoProof<[u8; 32]> = ChronoProof::new(0, 1000);
        assert_eq!(proof.leaf_index, 0);
        assert_eq!(proof.timestamp, 1000);
        assert!(proof.path.is_empty());
    }

    #[cfg(feature = "blake3-hash")]
    #[test]
    fn test_proof_steps() {
        let mut proof = ChronoProof::new(0, 1000);
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];

        proof.add_step(ProofStep::Left(hash1));
        proof.add_step(ProofStep::Right(hash2));

        assert_eq!(proof.path.len(), 2);
    }

    #[cfg(feature = "blake3-hash")]
    #[test]
    fn test_delta_chain() {
        let mut proof = ChronoProof::new(0, 1000);
        let delta = [3u8; 32];
        proof.add_delta(delta);

        assert!(proof.delta_chain.is_some());
        assert_eq!(proof.delta_chain.as_ref().unwrap().len(), 1);
    }
}
