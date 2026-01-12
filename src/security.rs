//! Security event logging and audit trails for ChronoMerkle trees
//!
//! This module provides security event logging capabilities for audit trails,
//! compliance monitoring, and security incident investigation.

use crate::error::Result;

#[cfg(feature = "no-std")]
use alloc::{string::String, vec::Vec};
#[cfg(not(feature = "no-std"))]
use std::vec::Vec;

/// Security event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SecurityLevel {
    /// Informational events (tree operations, configurations)
    Info,
    /// Warning events (potential security concerns)
    Warning,
    /// Critical security events (proof verification failures, tampering attempts)
    Critical,
}

/// Security event types
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SecurityEventType {
    /// Tree initialization
    TreeInitialization,
    /// Leaf insertion operation
    LeafInsertion,
    /// Proof generation
    ProofGeneration,
    /// Proof verification (successful)
    ProofVerificationSuccess,
    /// Proof verification failure
    ProofVerificationFailure,
    /// Configuration change
    ConfigurationChange,
    /// Potential tampering detected
    TamperingDetected,
    /// Input validation failure
    InputValidationFailure,
    /// Cryptographic operation failure
    CryptoOperationFailure,
}

/// Security event data
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SecurityEvent {
    /// Event timestamp (Unix timestamp)
    pub timestamp: u64,
    /// Event type
    pub event_type: SecurityEventType,
    /// Security severity level
    pub level: SecurityLevel,
    /// Human-readable description
    pub description: String,
    /// Event-specific metadata (optional)
    pub metadata: Option<SecurityMetadata>,
}

/// Additional metadata for security events
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SecurityMetadata {
    /// Leaf insertion metadata
    LeafInsertion {
        leaf_index: usize,
        timestamp: u64,
        data_hash: String, // Hex-encoded for logging
    },
    /// Proof verification metadata
    ProofVerification {
        leaf_index: usize,
        proof_timestamp: u64,
        verification_result: bool,
        failure_reason: Option<String>,
    },
    /// Configuration change metadata
    ConfigChange {
        parameter: String,
        old_value: String,
        new_value: String,
    },
    /// Validation failure metadata
    ValidationFailure {
        input_type: String,
        reason: String,
        input_value: Option<String>,
    },
}

/// Security logger trait for pluggable logging backends
pub trait SecurityLogger: Send + Sync {
    /// Log a security event
    fn log_event(&self, event: &SecurityEvent) -> Result<()>;

    /// Log multiple events (for batch operations)
    fn log_events(&self, events: &[SecurityEvent]) -> Result<()> {
        for event in events {
            self.log_event(event)?;
        }
        Ok(())
    }
}

/// No-op logger for when security logging is disabled
#[derive(Debug, Clone, Default)]
pub struct NoOpLogger;

impl SecurityLogger for NoOpLogger {
    fn log_event(&self, _event: &SecurityEvent) -> Result<()> {
        Ok(())
    }
}

#[cfg(feature = "std")]
/// Standard logger that writes to stderr
#[derive(Debug, Clone, Default)]
pub struct StdErrLogger;

#[cfg(feature = "std")]
impl SecurityLogger for StdErrLogger {
    fn log_event(&self, event: &SecurityEvent) -> Result<()> {
        eprintln!("[SECURITY] {} - {}: {}",
                 event.timestamp,
                 format!("{:?}", event.level).to_uppercase(),
                 event.description);
        Ok(())
    }
}

/// Helper functions for creating security events
pub mod events {
    use super::*;

    /// Create a tree initialization event
    pub fn tree_initialization(config_summary: &str) -> SecurityEvent {
        SecurityEvent {
            timestamp: current_timestamp(),
            event_type: SecurityEventType::TreeInitialization,
            level: SecurityLevel::Info,
            description: format!("ChronoMerkle tree initialized with config: {}", config_summary),
            metadata: None,
        }
    }

    /// Create a leaf insertion event
    pub fn leaf_insertion(leaf_index: usize, timestamp: u64, data_hash: &[u8]) -> SecurityEvent {
        SecurityEvent {
            timestamp: current_timestamp(),
            event_type: SecurityEventType::LeafInsertion,
            level: SecurityLevel::Info,
            description: format!("Leaf inserted at index {} with timestamp {}", leaf_index, timestamp),
            metadata: Some(SecurityMetadata::LeafInsertion {
                leaf_index,
                timestamp,
                data_hash: {
                    #[cfg(feature = "security-logging")]
                    {
                        hex::encode(data_hash)
                    }
                    #[cfg(not(feature = "security-logging"))]
                    {
                        format!("{:?}", data_hash) // Fallback when hex is not available
                    }
                },
            }),
        }
    }

    /// Create a proof generation event
    pub fn proof_generation(leaf_index: usize) -> SecurityEvent {
        SecurityEvent {
            timestamp: current_timestamp(),
            event_type: SecurityEventType::ProofGeneration,
            level: SecurityLevel::Info,
            description: format!("Merkle proof generated for leaf index {}", leaf_index),
            metadata: None,
        }
    }

    /// Create a successful proof verification event
    pub fn proof_verification_success(leaf_index: usize, proof_timestamp: u64) -> SecurityEvent {
        SecurityEvent {
            timestamp: current_timestamp(),
            event_type: SecurityEventType::ProofVerificationSuccess,
            level: SecurityLevel::Info,
            description: format!("Proof verification succeeded for leaf {} (proof timestamp: {})", leaf_index, proof_timestamp),
            metadata: Some(SecurityMetadata::ProofVerification {
                leaf_index,
                proof_timestamp,
                verification_result: true,
                failure_reason: None,
            }),
        }
    }

    /// Create a failed proof verification event
    pub fn proof_verification_failure(leaf_index: usize, proof_timestamp: u64, reason: &str) -> SecurityEvent {
        SecurityEvent {
            timestamp: current_timestamp(),
            event_type: SecurityEventType::ProofVerificationFailure,
            level: SecurityLevel::Critical,
            description: format!("PROOF VERIFICATION FAILED for leaf {}: {}", leaf_index, reason),
            metadata: Some(SecurityMetadata::ProofVerification {
                leaf_index,
                proof_timestamp,
                verification_result: false,
                failure_reason: Some(reason.to_string()),
            }),
        }
    }

    /// Create an input validation failure event
    pub fn input_validation_failure(input_type: &str, reason: &str, input_value: Option<&str>) -> SecurityEvent {
        SecurityEvent {
            timestamp: current_timestamp(),
            event_type: SecurityEventType::InputValidationFailure,
            level: SecurityLevel::Warning,
            description: format!("Input validation failed for {}: {}", input_type, reason),
            metadata: Some(SecurityMetadata::ValidationFailure {
                input_type: input_type.to_string(),
                reason: reason.to_string(),
                input_value: input_value.map(|s| s.to_string()),
            }),
        }
    }

    /// Create a configuration change event
    pub fn config_change(parameter: &str, old_value: &str, new_value: &str) -> SecurityEvent {
        SecurityEvent {
            timestamp: current_timestamp(),
            event_type: SecurityEventType::ConfigurationChange,
            level: SecurityLevel::Info,
            description: format!("Configuration changed: {} from '{}' to '{}'", parameter, old_value, new_value),
            metadata: Some(SecurityMetadata::ConfigChange {
                parameter: parameter.to_string(),
                old_value: old_value.to_string(),
                new_value: new_value.to_string(),
            }),
        }
    }
}

/// Get current timestamp (Unix timestamp)
/// This is a placeholder - in real implementation, this would get actual system time
#[cfg(feature = "std")]
pub fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(not(feature = "std"))]
pub fn current_timestamp() -> u64 {
    // In no-std environments, timestamp would need to be provided externally
    0
}

/// Constant-time equality comparison to prevent timing attacks
/// Returns true if both slices are equal, false otherwise
/// Time taken is independent of the data compared
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_event_creation() {
        let event = events::tree_initialization("test config");
        assert_eq!(event.event_type, SecurityEventType::TreeInitialization);
        assert_eq!(event.level, SecurityLevel::Info);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_stderr_logger() {
        let logger = StdErrLogger;
        let event = events::tree_initialization("test");
        // This should not panic
        logger.log_event(&event).unwrap();
    }

    #[test]
    fn test_noop_logger() {
        let logger = NoOpLogger;
        let event = events::tree_initialization("test");
        logger.log_event(&event).unwrap();
    }

    #[test]
    fn test_constant_time_eq() {
        // Test with equal arrays
        assert!(super::constant_time_eq(&[1, 2, 3], &[1, 2, 3]));

        // Test with different arrays
        assert!(!super::constant_time_eq(&[1, 2, 3], &[1, 2, 4]));

        // Test with different lengths
        assert!(!super::constant_time_eq(&[1, 2, 3], &[1, 2, 3, 4]));
        assert!(!super::constant_time_eq(&[1, 2, 3, 4], &[1, 2, 3]));

        // Test with empty arrays
        assert!(super::constant_time_eq(&[], &[]));

        // Test timing resistance (conceptual - hard to test directly)
        // Different arrays should take same time as equal arrays of same length
        let equal_arrays = ([0u8; 32], [0u8; 32]);
        let different_arrays = ([0u8; 32], [1u8; 32]);

        // Both should return results (timing would be tested with statistical analysis)
        assert!(super::constant_time_eq(&equal_arrays.0, &equal_arrays.1));
        assert!(!super::constant_time_eq(&different_arrays.0, &different_arrays.1));
    }
}