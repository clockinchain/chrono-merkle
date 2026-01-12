//! Proof generation and verification for ChronoMerkleTree

use crate::error::{ChronoMerkleError, Result};
use crate::hash::HashFunction;
use crate::tree::ChronoMerkleTree;

#[cfg(feature = "no-std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no-std"))]
use std::vec::Vec;

impl<H, Hasher, Logger> ChronoMerkleTree<H, Hasher, Logger>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: HashFunction<Output = H> + Sync,
    Logger: crate::security::SecurityLogger,
{
    /// Generate a proof for a leaf at the given index
    pub fn generate_proof(&self, leaf_index: usize) -> Result<crate::proof::ChronoProof<H>> {
        if leaf_index >= self.leaf_count {
            return Err(ChronoMerkleError::IndexOutOfBounds {
                index: leaf_index,
                leaf_count: self.leaf_count,
            });
        }

        let leaf = &self.nodes[leaf_index];
        let (timestamp, _) = leaf.timestamp_info();

        let mut proof = crate::proof::ChronoProof::new(leaf_index, timestamp);

        if self.leaf_count == 1 {
            // Single leaf, no proof path needed
            return Ok(proof);
        }

        // Build tree levels bottom-up
        let mut levels: Vec<Vec<H>> = vec![(0..self.leaf_count).map(|i| self.nodes[i].hash()).collect()];
        let mut current_level = levels[0].clone();
        let mut current_index = leaf_index;

        while current_level.len() > 1 {
            let mut next_level: Vec<H> = Vec::new();
            let next_index = current_index / 2;

            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    let parent_hash = self.hasher.hash_pair(&chunk[0], &chunk[1]);
                    next_level.push(parent_hash);
                } else {
                    // For odd number of nodes, duplicate the last node
                    let parent_hash = self.hasher.hash_pair(&chunk[0], &chunk[0]);
                    next_level.push(parent_hash);
                }
            }

            // Add sibling to proof path
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            let sibling_hash = if sibling_index < current_level.len() {
                current_level[sibling_index].clone()
            } else {
                // For odd number of nodes, duplicate the current node
                current_level[current_index].clone()
            };

            let step = if current_index % 2 == 0 {
                crate::proof::ProofStep::Right(sibling_hash)
            } else {
                crate::proof::ProofStep::Left(sibling_hash)
            };
            proof.add_step(step);

            levels.push(next_level.clone());
            current_level = next_level;
            current_index = next_index;
        }

        // Log proof generation
        let _ = self.security_logger.log_event(&crate::security::events::proof_generation(leaf_index));

        Ok(proof)
    }

    /// Verify a proof against the current root
    pub fn verify_proof(&self, proof: &crate::proof::ChronoProof<H>) -> Result<bool> {
        let root_hash = self.root().ok_or(ChronoMerkleError::EmptyTree)?;
        let leaf_hash = self.get_leaf_hash(proof.leaf_index)?;

        // Verify that proof timestamp matches the actual leaf timestamp
        let actual_timestamp = self.get_leaf_timestamp(proof.leaf_index)?;
        if proof.timestamp != actual_timestamp {
            // Log the verification failure
            let _ = self.security_logger.log_event(&crate::security::events::proof_verification_failure(
                proof.leaf_index,
                proof.timestamp,
                &format!("Proof timestamp mismatch: expected {}, got {}", actual_timestamp, proof.timestamp),
            ));
            return Ok(false);
        }

        let result = crate::proof::verify_proof(proof, &leaf_hash, &root_hash, &self.hasher)?;

        // Log proof verification result
        if result {
            let _ = self.security_logger.log_event(&crate::security::events::proof_verification_success(
                proof.leaf_index,
                proof.timestamp,
            ));
        } else {
            let _ = self.security_logger.log_event(&crate::security::events::proof_verification_failure(
                proof.leaf_index,
                proof.timestamp,
                "Proof verification failed - invalid proof or tampered data",
            ));
        }

        Ok(result)
    }
}