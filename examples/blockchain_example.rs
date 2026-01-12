//! # Blockchain Integration Example
//!
//! This example demonstrates how ChronoMerkle can be used for blockchain-like
//! operations including:
//! - Block creation with transactions
//! - Merkle tree commitment for blocks
//! - Proof generation for transaction inclusion
//! - Historical state verification

use chrono_merkle::{Blake3Hasher, DefaultChronoMerkleTree};

/// Represents a simple transaction
#[derive(Debug, Clone)]
struct Transaction {
    sender: String,
    receiver: String,
    amount: u64,
    data: Vec<u8>,
}

impl Transaction {
    fn new(sender: &str, receiver: &str, amount: u64) -> Self {
        Self {
            sender: sender.to_string(),
            receiver: receiver.to_string(),
            amount,
            data: format!("{}->{}:{}", sender, receiver, amount).into_bytes(),
        }
    }

    fn hash_data(&self) -> Vec<u8> {
        // Include all transaction details in the hash
        format!("{}:{}:{}:{:?}", self.sender, self.receiver, self.amount, self.data).into_bytes()
    }
}

/// Represents a blockchain block
#[derive(Debug, Clone)]
struct Block {
    height: u64,
    timestamp: u64,
    transactions: Vec<Transaction>,
    previous_hash: Option<[u8; 32]>,
    merkle_root: [u8; 32],
}

impl Block {
    fn new(height: u64, timestamp: u64, transactions: Vec<Transaction>, previous_hash: Option<[u8; 32]>) -> Self {
        // Create Merkle tree for transactions
        let mut tree = DefaultChronoMerkleTree::new(Blake3Hasher::default());

        // Insert transactions with their individual timestamps
        for (i, tx) in transactions.iter().enumerate() {
            let tx_timestamp = timestamp + i as u64;
            tree.insert(&tx.hash_data(), tx_timestamp).unwrap();
        }

        let merkle_root = tree.root().unwrap_or([0; 32]);

        Self {
            height,
            timestamp,
            transactions,
            previous_hash,
            merkle_root,
        }
    }

    fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    fn hash(&self) -> [u8; 32] {
        let mut tree = DefaultChronoMerkleTree::new(Blake3Hasher::default());

        // Include block header data in deterministic order
        tree.insert(&self.height.to_le_bytes(), self.timestamp).unwrap();
        tree.insert(&self.merkle_root, self.timestamp + 1).unwrap();

        if let Some(prev_hash) = self.previous_hash {
            tree.insert(&prev_hash, self.timestamp + 2).unwrap();
        }

        tree.root().unwrap_or([0; 32])
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("⛓️  ChronoMerkle Tree - Blockchain Integration Example\n");

    // Create a simple blockchain
    let mut blockchain: Vec<Block> = Vec::new();
    let mut previous_hash: Option<[u8; 32]> = None;

    // Create genesis block
    println!("🏆 Creating Genesis Block:");
    let genesis_transactions = vec![
        Transaction::new("faucet", "alice", 1000),
        Transaction::new("faucet", "bob", 1000),
    ];

    let genesis_block = Block::new(0, 1000000, genesis_transactions, None);
    let genesis_hash = genesis_block.hash();
    blockchain.push(genesis_block.clone());

    println!("  Block 0 - Genesis");
    println!("  Transactions: {}", genesis_block.transaction_count());
    println!("  Merkle Root: {:x}", genesis_block.merkle_root.iter().fold(0u64, |acc, &b| acc.wrapping_mul(256).wrapping_add(b as u64)));
    println!("  Block Hash: {:x}", genesis_hash.iter().fold(0u64, |acc, &b| acc.wrapping_mul(256).wrapping_add(b as u64)));

    previous_hash = Some(genesis_hash);

    // Create additional blocks
    let mut current_timestamp = 1000000;

    for block_num in 1..=3 {
        println!("\n📦 Creating Block {}:", block_num);

        // Create some transactions for this block
        let transactions = match block_num {
            1 => vec![
                Transaction::new("alice", "charlie", 100),
                Transaction::new("bob", "diana", 200),
            ],
            2 => vec![
                Transaction::new("charlie", "eve", 50),
            ],
            3 => vec![
                Transaction::new("diana", "alice", 150),
                Transaction::new("eve", "bob", 25),
                Transaction::new("alice", "frank", 75),
            ],
            _ => vec![],
        };

        current_timestamp += 1000; // Advance time
        let block = Block::new(block_num, current_timestamp, transactions.clone(), previous_hash);
        let block_hash = block.hash();

        println!("  Transactions: {}", transactions.len());
        println!("  Merkle Root: {:x}", block.merkle_root.iter().fold(0u64, |acc, &b| acc.wrapping_mul(256).wrapping_add(b as u64)));
        println!("  Block Hash: {:x}", block_hash.iter().fold(0u64, |acc, &b| acc.wrapping_mul(256).wrapping_add(b as u64)));

        // Verify transaction inclusion proof
        if !transactions.is_empty() {
            println!("  Verifying transaction inclusion:");
            let mut tree = DefaultChronoMerkleTree::new(Blake3Hasher::default());
            for (i, tx) in transactions.iter().enumerate() {
                tree.insert(&tx.hash_data(), current_timestamp + i as u64).unwrap();
            }

            let proof = tree.generate_proof(0)?;
            let is_valid = tree.verify_proof(&proof)?;
            println!("    Transaction 0 inclusion proof: {}", if is_valid { "✅ VALID" } else { "❌ INVALID" });
        }

        blockchain.push(block);
        previous_hash = Some(block_hash);
    }

    // Demonstrate historical verification
    println!("\n🔍 Historical Verification:");
    println!("Blockchain has {} blocks", blockchain.len());

    // Verify the entire chain
    let mut prev_hash: Option<[u8; 32]> = None;
    let mut valid_chain = true;

    for (i, block) in blockchain.iter().enumerate() {
        let block_hash = block.hash();

        if block.previous_hash != prev_hash {
            println!("❌ Block {}: Invalid previous hash", i);
            valid_chain = false;
        } else {
            println!("✅ Block {}: Valid link", i);
        }

        prev_hash = Some(block_hash);
    }

    println!("\n🎉 Blockchain example completed! Chain validity: {}",
             if valid_chain { "✅ VALID" } else { "❌ INVALID" });

    Ok(())
}