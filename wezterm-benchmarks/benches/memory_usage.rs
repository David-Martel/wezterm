use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use systemstat::{Platform, System};
use tokio::runtime::Runtime;
use wezterm_benchmarks::memory::{
    AllocationPattern, BufferPool, MemoryPool, MemoryTracker, ObjectPool,
};

fn get_memory_usage() -> u64 {
    let sys = System::new();
    if let Ok(mem) = sys.memory() {
        let total = mem.total.as_u64();
        let free = mem.free.as_u64();
        total - free
    } else {
        0
    }
}

fn bench_memory_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocations");

    // Test different allocation patterns
    group.bench_function("frequent_small_allocs", |b| {
        b.iter(|| {
            let mut vecs = Vec::new();
            for i in 0..1000 {
                let v = vec![i as u8; 128];
                vecs.push(v);
            }
            black_box(vecs)
        });
    });

    group.bench_function("pooled_small_allocs", |b| {
        let pool = BufferPool::new(1000, 128);
        b.iter(|| {
            let mut buffers = Vec::new();
            for _ in 0..1000 {
                let buf = pool.acquire();
                buffers.push(buf);
            }
            black_box(buffers)
        });
    });

    group.bench_function("large_single_alloc", |b| {
        b.iter(|| {
            let v = vec![0u8; 10_000_000];
            black_box(v)
        });
    });

    group.bench_function("chunked_large_alloc", |b| {
        b.iter(|| {
            let mut chunks = Vec::new();
            for _ in 0..100 {
                let chunk = vec![0u8; 100_000];
                chunks.push(chunk);
            }
            black_box(chunks)
        });
    });

    group.finish();
}

fn bench_object_pooling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("object_pooling");

    // Define a complex object for pooling
    #[derive(Clone)]
    struct ComplexObject {
        data: Vec<u8>,
        cache: std::collections::HashMap<String, String>,
        buffer: [u8; 4096],
    }

    impl Default for ComplexObject {
        fn default() -> Self {
            Self {
                data: Vec::with_capacity(1024),
                cache: std::collections::HashMap::with_capacity(100),
                buffer: [0; 4096],
            }
        }
    }

    group.bench_function("without_pool", |b| {
        b.iter(|| {
            let mut objects = Vec::new();
            for _ in 0..100 {
                let obj = ComplexObject::default();
                objects.push(obj);
            }
            black_box(objects)
        });
    });

    group.bench_function("with_pool", |b| {
        let pool = ObjectPool::<ComplexObject>::new(100);
        b.iter(|| {
            let mut objects = Vec::new();
            for _ in 0..100 {
                let obj = pool.acquire();
                objects.push(obj);
            }
            black_box(objects)
        });
    });

    group.bench_function("async_pool", |b| {
        b.to_async(&rt).iter(|| async {
            let pool = wezterm_benchmarks::memory::AsyncObjectPool::<ComplexObject>::new(100);
            let mut objects = Vec::new();
            for _ in 0..100 {
                let obj = pool.acquire().await;
                objects.push(obj);
            }
            black_box(objects)
        });
    });

    group.finish();
}

fn bench_memory_fragmentation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_fragmentation");
    group.sample_size(10);

    group.bench_function("fragmented_pattern", |b| {
        b.iter(|| {
            let mut allocations = Vec::new();

            // Create fragmented memory pattern
            for i in 0..1000 {
                if i % 2 == 0 {
                    allocations.push(vec![0u8; 1024]);
                } else {
                    allocations.push(vec![0u8; 512]);
                }
            }

            // Free every other allocation
            for i in (0..allocations.len()).step_by(2) {
                allocations[i].clear();
                allocations[i].shrink_to_fit();
            }

            // Allocate more
            for _ in 0..500 {
                allocations.push(vec![0u8; 768]);
            }

            black_box(allocations)
        });
    });

    group.bench_function("compacted_pattern", |b| {
        let pool = MemoryPool::new(2_000_000);
        b.iter(|| {
            let mut allocations = Vec::new();

            // Use memory pool for better allocation patterns
            for i in 0..1000 {
                let size = if i % 2 == 0 { 1024 } else { 512 };
                let alloc = pool.allocate(size);
                allocations.push(alloc);
            }

            // Pool handles deallocation more efficiently
            for i in (0..allocations.len()).step_by(2) {
                allocations[i].release();
            }

            // Allocate more from pool
            for _ in 0..500 {
                let alloc = pool.allocate(768);
                allocations.push(alloc);
            }

            black_box(allocations)
        });
    });

    group.finish();
}

fn bench_cache_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_memory");

    use lru::LruCache;
    use std::num::NonZeroUsize;

    group.bench_function("unlimited_cache", |b| {
        b.iter(|| {
            let mut cache = std::collections::HashMap::new();
            for i in 0..10000 {
                let key = format!("key_{}", i);
                let value = vec![0u8; 1024];
                cache.insert(key, value);
            }
            black_box(cache.len())
        });
    });

    group.bench_function("lru_cache", |b| {
        b.iter(|| {
            let mut cache = LruCache::<String, Vec<u8>>::new(NonZeroUsize::new(1000).unwrap());
            for i in 0..10000 {
                let key = format!("key_{}", i);
                let value = vec![0u8; 1024];
                cache.put(key, value);
            }
            black_box(cache.len())
        });
    });

    group.bench_function("size_limited_cache", |b| {
        b.iter(|| {
            let cache = wezterm_benchmarks::memory::SizeLimitedCache::new(1024 * 1024); // 1MB limit
            for i in 0..10000 {
                let key = format!("key_{}", i);
                let value = vec![0u8; 1024];
                cache.put(key, value);
            }
            black_box(cache.size())
        });
    });

    group.finish();
}

fn bench_memory_leaks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_leak_detection");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    group.bench_function("potential_leak", |b| {
        let tracker = MemoryTracker::new();
        b.iter(|| {
            // Simulate potential memory leak
            static mut LEAKED: Vec<Vec<u8>> = Vec::new();
            unsafe {
                for _ in 0..10 {
                    LEAKED.push(vec![0u8; 1024]);
                }
            }
            tracker.check_for_leak()
        });
    });

    group.bench_function("no_leak_with_cleanup", |b| {
        let tracker = MemoryTracker::new();
        b.iter(|| {
            let mut temp_storage = Vec::new();
            for _ in 0..10 {
                temp_storage.push(vec![0u8; 1024]);
            }
            // Proper cleanup
            temp_storage.clear();
            tracker.check_for_leak()
        });
    });

    group.bench_function("async_memory_tracking", |b| {
        b.to_async(&rt).iter(|| async {
            let tracker = wezterm_benchmarks::memory::AsyncMemoryTracker::new();

            let tasks: Vec<_> = (0..10)
                .map(|_| {
                    let t = tracker.clone();
                    tokio::spawn(async move {
                        let _data = vec![0u8; 10240];
                        t.record_allocation(10240).await;
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        t.record_deallocation(10240).await;
                    })
                })
                .collect();

            futures::future::join_all(tasks).await;
            tracker.get_current_usage().await
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_memory_allocations,
    bench_object_pooling,
    bench_memory_fragmentation,
    bench_cache_memory,
    bench_memory_leaks
);
criterion_main!(benches);
