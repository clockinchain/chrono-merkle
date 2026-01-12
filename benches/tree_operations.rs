use chrono_merkle::{ChronoMerkleTree, Blake3Hasher, security::NoOpLogger};
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_tree_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_construction");

    group.bench_function("create_empty_tree", |b| {
        b.iter(|| {
            let tree: ChronoMerkleTree<[u8; 32], Blake3Hasher, NoOpLogger> = ChronoMerkleTree::new(Blake3Hasher::default());
            std::hint::black_box(tree);
        });
    });

    group.bench_function("insert_1000_leaves", |b| {
        b.iter(|| {
            let mut tree: ChronoMerkleTree<[u8; 32], Blake3Hasher, NoOpLogger> = ChronoMerkleTree::new(Blake3Hasher::default());
            for i in 0..1000 {
                let data = format!("leaf_{}", i).into_bytes();
                tree.insert(&data, i as u64).unwrap();
            }
            std::hint::black_box(tree);
        });
    });

    group.bench_function("insert_10000_leaves", |b| {
        b.iter(|| {
            let mut tree: ChronoMerkleTree<[u8; 32], Blake3Hasher, NoOpLogger> = ChronoMerkleTree::new(Blake3Hasher::default());
            for i in 0..10000 {
                let data = format!("leaf_{}", i).into_bytes();
                tree.insert(&data, i as u64).unwrap();
            }
            std::hint::black_box(tree);
        });
    });

    group.finish();
}

fn bench_proof_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("proof_operations");

    // Setup tree with 1000 leaves
    let mut tree: ChronoMerkleTree<[u8; 32], Blake3Hasher, NoOpLogger> = ChronoMerkleTree::new(Blake3Hasher::default());
    for i in 0..1000 {
        let data = format!("leaf_{}", i).into_bytes();
        tree.insert(&data, i as u64).unwrap();
    }

    group.bench_function("generate_proof_small_tree", |b| {
        b.iter(|| {
            let proof = tree.generate_proof(500).unwrap();
            std::hint::black_box(proof);
        });
    });

    group.bench_function("verify_proof_small_tree", |b| {
        let proof = tree.generate_proof(500).unwrap();
        b.iter(|| {
            let result = tree.verify_proof(&proof).unwrap();
            std::hint::black_box(result);
        });
    });

    // Setup larger tree with 10000 leaves
    let mut large_tree: ChronoMerkleTree<[u8; 32], Blake3Hasher, NoOpLogger> = ChronoMerkleTree::new(Blake3Hasher::default());
    for i in 0..10000 {
        let data = format!("leaf_{}", i).into_bytes();
        large_tree.insert(&data, i as u64).unwrap();
    }

    group.bench_function("generate_proof_large_tree", |b| {
        b.iter(|| {
            let proof = large_tree.generate_proof(5000).unwrap();
            std::hint::black_box(proof);
        });
    });

    group.bench_function("verify_proof_large_tree", |b| {
        let proof = large_tree.generate_proof(5000).unwrap();
        b.iter(|| {
            let result = large_tree.verify_proof(&proof).unwrap();
            std::hint::black_box(result);
        });
    });

    group.finish();
}

fn bench_query_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_operations");

    // Setup tree with 10000 leaves at different timestamps
    let mut tree: ChronoMerkleTree<[u8; 32], Blake3Hasher, NoOpLogger> = ChronoMerkleTree::new(Blake3Hasher::default());
    for i in 0..10000 {
        let data = format!("data_{}", i).into_bytes();
        let timestamp = i as u64 * 1000; // Spread timestamps
        tree.insert(&data, timestamp).unwrap();
    }

    group.bench_function("exact_timestamp_lookup", |b| {
        b.iter(|| {
            let results = tree.find_by_timestamp(5000000); // Middle timestamp
            std::hint::black_box(results);
        });
    });

    group.bench_function("range_timestamp_query", |b| {
        b.iter(|| {
            let results = tree.find_range(2000000, 8000000); // 20% to 80% range
            std::hint::black_box(results);
        });
    });

    group.bench_function("small_range_query", |b| {
        b.iter(|| {
            let results = tree.find_range(5000000, 5010000); // Small range
            std::hint::black_box(results);
        });
    });

    group.finish();
}

fn bench_tree_properties(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_properties");

    // Setup tree with 1000 leaves
    let mut tree: ChronoMerkleTree<[u8; 32], Blake3Hasher, NoOpLogger> = ChronoMerkleTree::new(Blake3Hasher::default());
    for i in 0..1000 {
        let data = format!("leaf_{}", i).into_bytes();
        tree.insert(&data, i as u64).unwrap();
    }

    group.bench_function("get_root_hash", |b| {
        b.iter(|| {
            let root = tree.root();
            std::hint::black_box(root);
        });
    });

    group.bench_function("get_tree_depth", |b| {
        b.iter(|| {
            let depth = tree.depth();
            std::hint::black_box(depth);
        });
    });

    group.bench_function("get_leaf_count", |b| {
        b.iter(|| {
            let count = tree.leaf_count();
            std::hint::black_box(count);
        });
    });

    group.finish();
}

fn bench_incremental_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_updates");

    // Start with a tree that has some data
    let mut tree: ChronoMerkleTree<[u8; 32], Blake3Hasher, NoOpLogger> = ChronoMerkleTree::new(Blake3Hasher::default());
    for i in 0..100 {
        let data = format!("initial_{}", i).into_bytes();
        tree.insert(&data, i as u64).unwrap();
    }

    group.bench_function("single_insert", |b| {
        b.iter(|| {
            let mut test_tree = tree.clone();
            let data = b"new_leaf_data";
            test_tree.insert(data, 999).unwrap();
            std::hint::black_box(test_tree);
        });
    });

    group.bench_function("batch_insert_10", |b| {
        b.iter(|| {
            let mut test_tree = tree.clone();
            for i in 0..10 {
                let data = format!("batch_{}", i).into_bytes();
                test_tree.insert(&data, 1000 + i as u64).unwrap();
            }
            std::hint::black_box(test_tree);
        });
    });

    group.bench_function("batch_insert_100", |b| {
        b.iter(|| {
            let mut test_tree = tree.clone();
            for i in 0..100 {
                let data = format!("batch_{}", i).into_bytes();
                test_tree.insert(&data, 1000 + i as u64).unwrap();
            }
            std::hint::black_box(test_tree);
        });
    });

    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    group.bench_function("tree_memory_overhead", |b| {
        b.iter(|| {
            // Measure memory overhead of empty tree
            let tree = ChronoMerkleTree::<[u8; 32], Blake3Hasher>::new(Blake3Hasher::default());
            std::hint::black_box(tree);
        });
    });

    group.bench_function("scaling_memory_usage", |b| {
        b.iter(|| {
            let mut tree: ChronoMerkleTree<[u8; 32], Blake3Hasher, NoOpLogger> = ChronoMerkleTree::new(Blake3Hasher::default());
            // Add increasing numbers of leaves and measure memory scaling
            for i in 0..100 {
                let data = [i as u8; 32]; // Fixed-size data
                tree.insert(&data, i as u64).unwrap();
            }
            std::hint::black_box(tree);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_tree_construction,
    bench_proof_operations,
    bench_query_operations,
    bench_tree_properties,
    bench_incremental_updates,
    bench_memory_usage
);

criterion_main!(benches);