use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use wezterm_benchmarks::ipc::{ConnectionPool, IpcClient, IpcMessage, MessageBatcher};

fn bench_ipc_roundtrip(c: &mut Criterion) {
    let rt = Runtime::new().expect("create tokio runtime for IPC roundtrip benchmark");

    let mut group = c.benchmark_group("ipc_roundtrip");
    group.measurement_time(Duration::from_secs(10));

    // Test different payload sizes
    for size in [100, 1_000, 10_000, 100_000, 1_000_000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::new("json", size), size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let client = IpcClient::connect_json()
                    .await
                    .expect("connect JSON IPC client for roundtrip benchmark");
                let payload = vec![0u8; size];

                let start = Instant::now();
                let _response = client
                    .send_request("echo", &payload)
                    .await
                    .expect("send JSON IPC echo request");
                black_box(start.elapsed())
            });
        });

        group.bench_with_input(BenchmarkId::new("msgpack", size), size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let client = IpcClient::connect_msgpack()
                    .await
                    .expect("connect msgpack IPC client for roundtrip benchmark");
                let payload = vec![0u8; size];

                let start = Instant::now();
                let _response = client
                    .send_request("echo", &payload)
                    .await
                    .expect("send msgpack IPC echo request");
                black_box(start.elapsed())
            });
        });

        group.bench_with_input(BenchmarkId::new("msgpack_lz4", size), size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let client = IpcClient::connect_compressed()
                    .await
                    .expect("connect msgpack+lz4 IPC client for roundtrip benchmark");
                let payload = vec![0u8; size];

                let start = Instant::now();
                let _response = client
                    .send_request("echo", &payload)
                    .await
                    .expect("send msgpack+lz4 IPC echo request");
                black_box(start.elapsed())
            });
        });
    }

    group.finish();
}

fn bench_connection_pooling(c: &mut Criterion) {
    let rt = Runtime::new().expect("create tokio runtime for connection pooling benchmark");

    let mut group = c.benchmark_group("connection_pooling");

    group.bench_function("without_pool", |b| {
        b.to_async(&rt).iter(|| async {
            let client = IpcClient::connect_json()
                .await
                .expect("connect JSON IPC client without pool");
            let _response = client
                .send_request("ping", &())
                .await
                .expect("send ping without connection pool");
            black_box(client)
        });
    });

    group.bench_function("with_pool", |b| {
        let pool = rt.block_on(async { ConnectionPool::new(10).await });

        b.to_async(&rt).iter(|| async {
            let client = pool.get_or_create("test").await;
            let _response = client
                .send_request("ping", &())
                .await
                .expect("send ping via connection pool");
            black_box(client)
        });
    });

    group.finish();
}

fn bench_message_batching(c: &mut Criterion) {
    let rt = Runtime::new().expect("create tokio runtime for message batching benchmark");

    let mut group = c.benchmark_group("message_batching");

    // Test sending 100 small messages
    group.bench_function("sequential_100", |b| {
        b.to_async(&rt).iter(|| async {
            let client = IpcClient::connect_json()
                .await
                .expect("connect JSON IPC client for sequential benchmark");
            for i in 0..100 {
                let _response = client
                    .send_request("echo", &i)
                    .await
                    .expect("send sequential echo request");
            }
        });
    });

    group.bench_function("batched_100", |b| {
        b.to_async(&rt).iter(|| async {
            let client = IpcClient::connect_json()
                .await
                .expect("connect JSON IPC client for batching benchmark");
            let mut batcher = MessageBatcher::new(client);

            let futures: Vec<_> = (0..100).map(|i| batcher.send("echo", i)).collect();

            let _results = futures::future::join_all(futures).await;
        });
    });

    group.finish();
}

fn bench_concurrent_clients(c: &mut Criterion) {
    let rt = Runtime::new().expect("create tokio runtime for concurrent clients benchmark");

    let mut group = c.benchmark_group("concurrent_clients");
    group.sample_size(10);

    for num_clients in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_requests", num_clients),
            num_clients,
            |b, &num_clients| {
                b.to_async(&rt).iter(|| async move {
                    let futures: Vec<_> = (0..num_clients)
                        .map(|_| async {
                            let client = IpcClient::connect_json()
                                .await
                                .expect("connect JSON IPC client for concurrent benchmark");
                            for _ in 0..10 {
                                let _response = client
                                    .send_request("ping", &())
                                    .await
                                    .expect("send ping in concurrent benchmark");
                            }
                        })
                        .collect();

                    futures::future::join_all(futures).await
                });
            },
        );
    }

    group.finish();
}

fn bench_zero_copy(c: &mut Criterion) {
    let rt = Runtime::new().expect("create tokio runtime for zero-copy benchmark");

    let mut group = c.benchmark_group("zero_copy");

    let data = vec![0u8; 1_000_000];
    let bytes = Bytes::from(data.clone());

    group.bench_function("regular_copy", |b| {
        b.to_async(&rt).iter(|| async {
            let client = IpcClient::connect_json()
                .await
                .expect("connect JSON IPC client for copy benchmark");
            let _response = client
                .send_request("echo", &data)
                .await
                .expect("send regular copy echo request");
        });
    });

    group.bench_function("zero_copy", |b| {
        b.to_async(&rt).iter(|| async {
            let client = IpcClient::connect_zero_copy()
                .await
                .expect("connect zero-copy IPC client for benchmark");
            let _response = client
                .send_bytes("echo", bytes.clone())
                .await
                .expect("send zero-copy bytes echo request");
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_ipc_roundtrip,
    bench_connection_pooling,
    bench_message_batching,
    bench_concurrent_clients,
    bench_zero_copy
);
criterion_main!(benches);
