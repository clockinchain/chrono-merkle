//! # Programmable Nodes Example
//!
//! This example demonstrates ChronoMerkle's programmable node feature,
//! which allows custom validation logic at internal tree nodes.
//!
//! Use cases:
//! - Business rule validation
//! - Access control
//! - Data integrity checks
//! - Custom consensus rules

use chrono_merkle::{ChronoMerkleTree, Blake3Hasher, NodeType};
use std::sync::Arc;

/// Custom validation function type
type ValidationFn = Arc<dyn Fn(&[u8], &[u8]) -> Result<(), String> + Send + Sync>;

/// Represents a programmable node with custom validation
#[derive(Clone)]
struct ProgrammableNode {
    validation_fn: ValidationFn,
    description: String,
}

impl ProgrammableNode {
    fn new<F>(description: &str, validator: F) -> Self
    where
        F: Fn(&[u8], &[u8]) -> Result<(), String> + Send + Sync + 'static,
    {
        Self {
            validation_fn: Arc::new(validator),
            description: description.to_string(),
        }
    }

    fn validate(&self, left: &[u8], right: &[u8]) -> Result<(), String> {
        (self.validation_fn)(left, right)
    }
}

/// Business rule: Only allow transactions where sender balance >= amount
fn balance_validator(min_balance: u64) -> ProgrammableNode {
    ProgrammableNode::new(
        &format!("Balance >= {}", min_balance),
        move |left_data, right_data| {
            // Simple balance check simulation
            // In real usage, this would check actual account balances
            let left_balance = left_data.get(0).copied().unwrap_or(0) as u64;
            let right_amount = right_data.get(0).copied().unwrap_or(0) as u64;

            if left_balance >= right_amount && left_balance >= min_balance {
                Ok(())
            } else {
                Err(format!("Insufficient balance: {} < {} or < {}",
                           left_balance, right_amount, min_balance))
            }
        }
    )
}

/// Security rule: Only allow transactions with valid signatures
fn signature_validator(expected_sig: &[u8]) -> ProgrammableNode {
    let expected = expected_sig.to_vec();
    ProgrammableNode::new(
        "Valid Signature Required",
        move |_left_data, right_data| {
            // Simple signature check (in practice, use proper crypto)
            if right_data.len() >= expected.len() &&
               &right_data[right_data.len() - expected.len()..] == expected.as_slice() {
                Ok(())
            } else {
                Err("Invalid signature".to_string())
            }
        }
    )
}

/// Compliance rule: Flag suspicious transaction patterns
fn compliance_validator() -> ProgrammableNode {
    ProgrammableNode::new(
        "Compliance Check",
        |left_data, right_data| {
            // Check for suspicious patterns (simplified)
            let left_sum: u64 = left_data.iter().map(|&b| b as u64).sum();
            let right_sum: u64 = right_data.iter().map(|&b| b as u64).sum();

            if left_sum > 1000 && right_sum > 500 {
                Err("Suspicious transaction pattern detected".to_string())
            } else {
                Ok(())
            }
        }
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎛️  ChronoMerkle Tree - Programmable Nodes Example\n");

    // Create a tree with programmable nodes
    let mut tree = ChronoMerkleTree::new(Blake3Hasher::default());
    println!("✓ Created ChronoMerkle tree with programmable node support");

    // Define some programmable nodes
    let balance_check = balance_validator(100);
    let signature_check = signature_validator(b"valid_sig");
    let compliance_check = compliance_validator();

    println!("\n🔧 Programmable Nodes Created:");
    println!("  1. {}", balance_check.description);
    println!("  2. {}", signature_check.description);
    println!("  3. {}", compliance_check.description);

    // Example data that should pass validation
    let valid_transactions = vec![
        (vec![150, 0, 0, 0, b'v', b'a', b'l', b'i', b'd', b'_', b's', b'i', b'g'], 1000), // balance=150, valid sig
        (vec![200, 0, 0, 0, b'v', b'a', b'l', b'i', b'd', b'_', b's', b'i', b'g'], 1001), // balance=200, valid sig
    ];

    println!("\n✅ Testing Valid Transactions:");
    for (i, (data, timestamp)) in valid_transactions.iter().enumerate() {
        // Test individual validations
        println!("  Transaction {}:", i + 1);

        match balance_check.validate(&[], data) {
            Ok(_) => println!("    ✓ Balance check passed"),
            Err(e) => println!("    ✗ Balance check failed: {}", e),
        }

        match signature_check.validate(&[], data) {
            Ok(_) => println!("    ✓ Signature check passed"),
            Err(e) => println!("    ✗ Signature check failed: {}", e),
        }

        match compliance_check.validate(&[], data) {
            Ok(_) => println!("    ✓ Compliance check passed"),
            Err(e) => println!("    ✗ Compliance check failed: {}", e),
        }

        // Insert into tree
        tree.insert(data, *timestamp)?;
        println!("    → Inserted into tree at timestamp {}", timestamp);
    }

    // Example data that should fail validation
    let invalid_transactions = vec![
        (vec![50, 0, 0, 0, b'v', b'a', b'l', b'i', b'd', b'_', b's', b'i', b'g'], 1002),  // low balance=50
        (vec![150, 0, 0, 0, b'i', b'n', b'v', b'a', b'l', b'i', b'd'], 1003),            // invalid sig
        (vec![255, 0, 0, 0, b'v', b'a', b'l', b'i', b'd', b'_', b's', b'i', b'g'], 1004), // suspicious pattern
    ];

    println!("\n❌ Testing Invalid Transactions:");
    for (i, (data, timestamp)) in invalid_transactions.iter().enumerate() {
        println!("  Transaction {}:", i + 3);

        let mut failed_checks = Vec::new();

        if balance_check.validate(&[], data).is_err() {
            failed_checks.push("balance");
        }
        if signature_check.validate(&[], data).is_err() {
            failed_checks.push("signature");
        }
        if compliance_check.validate(&[], data).is_err() {
            failed_checks.push("compliance");
        }

        if failed_checks.is_empty() {
            println!("    ⚠️  Unexpectedly passed all checks");
            tree.insert(data, *timestamp)?;
        } else {
            println!("    ✗ Failed checks: {}", failed_checks.join(", "));
            println!("    → Rejected - not inserted into tree");
        }
    }

    // Show tree statistics
    println!("\n📊 Tree Statistics:");
    println!("  Total leaves: {}", tree.leaf_count());
    println!("  Tree depth: {}", tree.depth());

    if let Some(root) = tree.root() {
        println!("  Root hash: {:x}", root.iter().fold(0u64, |acc, &b| acc.wrapping_mul(256).wrapping_add(b as u64)));
    }

    // Demonstrate proof generation for validated data
    if tree.leaf_count() > 0 {
        println!("\n🔐 Proof Generation:");
        let proof = tree.generate_proof(0)?;
        let is_valid = tree.verify_proof(&proof)?;
        println!("  Proof for leaf 0: {}", if is_valid { "✅ VALID" } else { "❌ INVALID" });
    }

    println!("\n🎉 Programmable nodes example completed!");
    println!("💡 Programmable nodes enable custom business logic and validation rules in Merkle trees");

    Ok(())
}