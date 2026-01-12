//! Input validation methods for ChronoMerkleTree

use crate::error::{ChronoMerkleError, Result};
use crate::security::SecurityLogger;
use crate::tree::ChronoMerkleTree;
use crate::hash::HashFunction;

#[cfg(feature = "no-std")]
use alloc::string::ToString;
#[cfg(not(feature = "no-std"))]
use std::string::ToString;

impl<H, Hasher, Logger> ChronoMerkleTree<H, Hasher, Logger>
where
    H: AsRef<[u8]> + Clone + Eq + core::fmt::Debug + Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
    Hasher: HashFunction<Output = H> + Sync,
    Logger: SecurityLogger,
{
    /// Validate inputs for insert operation
    pub(crate) fn validate_insert_inputs(&self, data: &[u8], timestamp: u64) -> Result<()> {
        // SECURITY: Validate data size to prevent DoS through excessive memory usage
        const MAX_DATA_SIZE: usize = 1024 * 1024; // 1MB limit
        if data.len() > MAX_DATA_SIZE {
            let _ = self.security_logger.log_event(&crate::security::events::input_validation_failure(
                "data",
                &format!("Data size {} exceeds maximum allowed size {}", data.len(), MAX_DATA_SIZE),
                None,
            ));
            return Err(ChronoMerkleError::InvalidConfiguration {
                parameter: "data".to_string(),
                reason: format!("Data size {} exceeds maximum allowed size {}", data.len(), MAX_DATA_SIZE),
            });
        }

        // SECURITY: Validate data is not empty (empty data could cause issues)
        if data.is_empty() {
            let _ = self.security_logger.log_event(&crate::security::events::input_validation_failure(
                "data",
                "Empty data not allowed",
                Some(""),
            ));
            return Err(ChronoMerkleError::InvalidConfiguration {
                parameter: "data".to_string(),
                reason: "Empty data not allowed".to_string(),
            });
        }

        // SECURITY: Validate timestamp is reasonable (not in far future or past)
        // Allow timestamps up to 1 year in the future and 100 years in the past
        let current_time = crate::security::current_timestamp();
        let one_year_future = current_time + (365 * 24 * 60 * 60);
        // Prevent underflow in test environments where current_time might be 0
        let hundred_years_ago = 100 * 365 * 24 * 60 * 60;
        let hundred_years_past = current_time.saturating_sub(hundred_years_ago);

        if timestamp > one_year_future {
            let _ = self.security_logger.log_event(&crate::security::events::input_validation_failure(
                "timestamp",
                &format!("Timestamp {} is too far in the future (current: {})", timestamp, current_time),
                Some(&timestamp.to_string()),
            ));
            return Err(ChronoMerkleError::InvalidTimestamp { timestamp });
        }

        if timestamp < hundred_years_past {
            let _ = self.security_logger.log_event(&crate::security::events::input_validation_failure(
                "timestamp",
                &format!("Timestamp {} is too far in the past (current: {})", timestamp, current_time),
                Some(&timestamp.to_string()),
            ));
            return Err(ChronoMerkleError::InvalidTimestamp { timestamp });
        }

        // SECURITY: Check for duplicate timestamps (could indicate replay attacks)
        if self.sparse_index.find_exact(timestamp).is_some() {
            let _ = self.security_logger.log_event(&crate::security::events::input_validation_failure(
                "timestamp",
                &format!("Duplicate timestamp {} detected", timestamp),
                Some(&timestamp.to_string()),
            ));
            // Note: We allow duplicate timestamps for now but log it as a warning
            // In production systems, you might want to reject duplicates
        }

        Ok(())
    }
}