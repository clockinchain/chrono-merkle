use chrono_merkle::{ChronoMerkleTree, Blake3Hasher, security::NoOpLogger};
use criterion::{criterion_group, criterion_main, Criterion};

#[cfg(feature = "clockhash")]
use chrono_merkle::ClockHashAdapter;

#[cfg(feature = "clockhash")]
fn bench_clockhash_adapter(c: &mut Criterion) {
    let mut group = c.benchmark_group("clockhash_adapter");

    group.bench_function("create_adapter", |b| {
        b.iter(|| {
            let adapter = ClockHashAdapter::new(1000);
            std::hint::black_box(adapter);
        });
    });

    group.bench_function("add_single_trace_entry", |b| {
        b.iter(|| {
            let mut adapter = ClockHashAdapter::new(1000);
            let trace_data = b"operation_data";
            adapter.add_trace_block(trace_data).unwrap();
            std::hint::black_box(adapter);
        });
    });

    group.bench_function("add_100_trace_entries", |b| {
        b.iter(|| {
            let mut adapter = ClockHashAdapter::new(1000);
            for i in 0..100 {
                let trace_data = format!("operation_{}", i).into_bytes();
                adapter.add_trace_block(&trace_data).unwrap();
            }
            std::hint::black_box(adapter);
        });
    });

    group.bench_function("compute_trace_root", |b| {
        let mut adapter = ClockHashAdapter::new(1000);
        for i in 0..100 {
            let trace_data = format!("operation_{}", i).into_bytes();
            adapter.add_trace_block(&trace_data).unwrap();
        }

        b.iter(|| {
            let root = adapter.compute_trace_root().unwrap();
            std::hint::black_box(root);
        });
    });

    group.bench_function("timestamp_queries", |b| {
        let mut adapter = ClockHashAdapter::new(1000);
        for i in 0..1000 {
            let trace_data = format!("operation_{}", i).into_bytes();
            adapter.add_trace_block(&trace_data).unwrap();
        }

        b.iter(|| {
            let results = adapter.query_by_time_slice(1000);
            std::hint::black_box(results);
        });
    });

    group.finish();
}

#[cfg(feature = "clockhash")]
fn bench_trace_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("trace_compression");

    // Generate a realistic trace pattern
    let trace_entries = generate_realistic_trace(1000);

    group.bench_function("compress_1000_trace_entries", |b| {
        b.iter(|| {
            let mut adapter = ClockHashAdapter::new(1000);
            for entry in &trace_entries {
                adapter.add_trace_block(entry).unwrap();
            }
            std::hint::black_box(adapter.compute_trace_root().unwrap());
        });
    });

    group.bench_function("compress_10000_trace_entries", |b| {
        let large_trace = generate_realistic_trace(10000);
        b.iter(|| {
            let mut adapter = ClockHashAdapter::new(1000);
            for entry in &large_trace {
                adapter.add_trace_block(entry).unwrap();
            }
            std::hint::black_box(adapter.compute_trace_root().unwrap());
        });
    });

    // Compare with regular ChronoMerkle tree
    group.bench_function("regular_tree_1000_entries", |b| {
        b.iter(|| {
            let mut tree: ChronoMerkleTree<[u8; 32], Blake3Hasher, NoOpLogger> = ChronoMerkleTree::new(Blake3Hasher::default());
            for (i, entry) in trace_entries.iter().enumerate() {
                tree.insert(entry, 1000 + i as u64).unwrap();
            }
            std::hint::black_box(tree.root().unwrap());
        });
    });

    group.finish();
}

#[cfg(feature = "clockhash")]
fn bench_incremental_trace_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_trace_building");

    group.bench_function("incremental_vs_batch_100", |b| {
        // Incremental building
        b.iter(|| {
            let mut adapter = ClockHashAdapter::new(1000);
            for i in 0..100 {
                let trace_data = format!("op_{}", i).into_bytes();
                adapter.add_trace_block(&trace_data).unwrap();
                // Compute root at each step (simulating incremental verification)
                std::hint::black_box(adapter.compute_trace_root().unwrap());
            }
        });
    });

    group.bench_function("batch_building_100", |b| {
        b.iter(|| {
            let mut adapter = ClockHashAdapter::new(1000);
            for i in 0..100 {
                let trace_data = format!("op_{}", i).into_bytes();
                adapter.add_trace_block(&trace_data).unwrap();
            }
            // Single root computation at the end
            std::hint::black_box(adapter.compute_trace_root().unwrap());
        });
    });

    group.finish();
}

#[cfg(feature = "clockhash")]
fn generate_realistic_trace(size: usize) -> Vec<Vec<u8>> {
    let mut trace = Vec::with_capacity(size);

    for i in 0..size {
        // Simulate different types of operations with varying data sizes
        let operation = match i % 10 {
            0 => format!("load_var_{}", i),
            1 => format!("store_var_{}", i),
            2 => format!("call_func_{}", i),
            3 => format!("return_val_{}", i),
            4 => format!("jump_cond_{}", i),
            5 => format!("arith_add_{}", i),
            6 => format!("arith_mul_{}", i),
            7 => format!("memory_read_{}", i),
            8 => format!("memory_write_{}", i),
            9 => format!("branch_{}", i),
            _ => format!("unknown_{}", i),
        };

        let data = if i % 5 == 0 {
            // Larger data occasionally
            format!("large_data_{}_with_additional_context_and_metadata", operation).into_bytes()
        } else {
            operation.into_bytes()
        };

        trace.push(data);
    }

    trace
}

#[cfg(feature = "clockhash")]
criterion_group!(
    benches,
    bench_clockhash_adapter,
    bench_trace_compression,
    bench_incremental_trace_building
);

#[cfg(not(feature = "clockhash"))]
fn bench_placeholder(c: &mut Criterion) {
    let mut group = c.benchmark_group("clockhash_placeholder");

    group.bench_function("feature_not_enabled", |b| {
        b.iter(|| {
            // Placeholder benchmark when clockhash feature is not enabled
            std::hint::black_box(());
        });
    });

    group.finish();
}

#[cfg(not(feature = "clockhash"))]
criterion_group!(benches, bench_placeholder);

criterion_main!(benches);