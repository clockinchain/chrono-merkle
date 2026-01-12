//! Trait definitions for extensibility

/// Trait for types that can be used as timestamps
pub trait Timestamp: Clone + Copy + PartialOrd + Ord + core::fmt::Debug {
    /// Convert to u64 representation
    fn to_u64(self) -> u64;
    
    /// Create from u64 representation
    fn from_u64(value: u64) -> Self;
}

impl Timestamp for u64 {
    fn to_u64(self) -> u64 {
        self
    }
    
    fn from_u64(value: u64) -> Self {
        value
    }
}

/// Trait for types that can be used as hash outputs
pub trait HashOutput: AsRef<[u8]> + Clone + Eq + core::fmt::Debug {
    /// Get the hash as bytes
    fn as_bytes(&self) -> &[u8];
    
    /// Create from bytes
    fn from_bytes(bytes: &[u8]) -> Option<Self>;
}

impl HashOutput for [u8; 32] {
    fn as_bytes(&self) -> &[u8] {
        self
    }
    
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == 32 {
            let mut array = [0u8; 32];
            array.copy_from_slice(bytes);
            Some(array)
        } else {
            None
        }
    }
}
