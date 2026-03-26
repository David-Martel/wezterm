use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use walkdir::WalkDir;
use wezterm_benchmarks::fs::{
    DirectoryScanner, FileCache, IncrementalScanner, MemoryMappedReader, ParallelScanner,
};

fn create_test_directory(num_files: usize, depth: usize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();

    // Create a directory structure with the specified number of files and depth
    for i in 0..num_files {
        let mut path = base_path.to_path_buf();

        // Create nested directories based on file index
        for d in 0..depth {
            path.push(format!("dir_{}", (i / 100) % 10 + d));
        }

        std::fs::create_dir_all(&path).ok();

        let file_path = path.join(format!("file_{}.txt", i));
        std::fs::write(&file_path, format!("Content of file {}", i)).unwrap();
    }

    temp_dir
}

fn bench_directory_scanning(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("directory_scanning");
    group.sample_size(10);

    for &num_files in &[100, 1_000, 10_000] {
        let test_dir = create_test_directory(num_files, 3);
        let path = test_dir.path().to_path_buf();

        group.throughput(Throughput::Elements(num_files as u64));

        group.bench_with_input(BenchmarkId::new("walkdir", num_files), &path, |b, path| {
            b.iter(|| {
                let count = WalkDir::new(path)
                    .into_iter()
                    .filter_map(Result::ok)
                    .count();
                black_box(count)
            });
        });

        group.bench_with_input(
            BenchmarkId::new("parallel_scan", num_files),
            &path,
            |b, path| {
                b.to_async(&rt).iter(|| async {
                    let scanner = ParallelScanner::new();
                    let entries = scanner.scan(path).await.unwrap();
                    black_box(entries.len())
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("cached_scan", num_files),
            &path,
            |b, path| {
                let scanner = DirectoryScanner::with_cache(1000);
                b.iter(|| {
                    let entries = scanner.scan_cached(path);
                    black_box(entries.len())
                });
            },
        );
    }

    group.finish();
}

fn bench_incremental_updates(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("incremental_updates");

    let test_dir = create_test_directory(1_000, 2);
    let path = test_dir.path().to_path_buf();

    group.bench_function("full_refresh", |b| {
        b.to_async(&rt).iter(|| async {
            let scanner = DirectoryScanner::new();
            let entries = scanner.scan(&path).await.unwrap();
            black_box(entries)
        });
    });

    group.bench_function("incremental_update", |b| {
        let scanner = rt.block_on(async {
            let s = IncrementalScanner::new();
            s.initial_scan(&path).await.unwrap();
            s
        });

        b.to_async(&rt).iter(|| async {
            let changes = scanner.get_changes(&path).await.unwrap();
            black_box(changes)
        });
    });

    group.finish();
}

fn bench_file_reading(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("file_reading");

    let test_dir = TempDir::new().unwrap();

    // Create files of different sizes
    let sizes = vec![
        (1_024, "1KB"),
        (100_024, "100KB"),
        (1_024_000, "1MB"),
        (10_024_000, "10MB"),
    ];

    for (size, label) in sizes {
        let file_path = test_dir.path().join(format!("test_{}.bin", label));
        let data = vec![0u8; size];
        std::fs::write(&file_path, &data).unwrap();

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("std_read", label),
            &file_path,
            |b, path| {
                b.iter(|| {
                    let content = std::fs::read(path).unwrap();
                    black_box(content)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("tokio_read", label),
            &file_path,
            |b, path| {
                b.to_async(&rt).iter(|| async {
                    let content = tokio::fs::read(path).await.unwrap();
                    black_box(content)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("mmap_read", label),
            &file_path,
            |b, path| {
                b.iter(|| {
                    let reader = MemoryMappedReader::new(path).unwrap();
                    let content = reader.read_all();
                    black_box(content)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("cached_read", label),
            &file_path,
            |b, path| {
                let cache = FileCache::new(100);
                b.iter(|| {
                    let content = cache.read(path).unwrap();
                    black_box(content)
                });
            },
        );
    }

    group.finish();
}

fn bench_file_watching(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("file_watching");
    group.sample_size(10);

    let test_dir = create_test_directory(100, 2);
    let path = test_dir.path().to_path_buf();

    group.bench_function("event_processing", |b| {
        b.to_async(&rt).iter(|| async {
            let watcher = wezterm_benchmarks::fs::Watcher::new();
            watcher.watch(&path).await.unwrap();

            // Simulate file changes
            for i in 0..10 {
                let file = path.join(format!("change_{}.txt", i));
                tokio::fs::write(&file, format!("change {}", i))
                    .await
                    .unwrap();
            }

            // Process events
            let events = watcher.get_events().await;
            black_box(events)
        });
    });

    group.bench_function("debounced_events", |b| {
        b.to_async(&rt).iter(|| async {
            let watcher =
                wezterm_benchmarks::fs::DebouncedWatcher::new(std::time::Duration::from_millis(50));
            watcher.watch(&path).await.unwrap();

            // Simulate rapid file changes
            for i in 0..50 {
                let file = path.join(format!("rapid_{}.txt", i));
                tokio::fs::write(&file, format!("change {}", i))
                    .await
                    .unwrap();
            }

            // Wait for debouncing
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // Process debounced events
            let events = watcher.get_debounced_events().await;
            black_box(events)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_directory_scanning,
    bench_incremental_updates,
    bench_file_reading,
    bench_file_watching
);
criterion_main!(benches);
