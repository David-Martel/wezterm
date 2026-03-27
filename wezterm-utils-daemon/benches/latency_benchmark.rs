//! Performance benchmarks for wezterm-utils-daemon

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;
use std::time::Duration;
use tokio::runtime::Runtime;

#[cfg(windows)]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(windows)]
use tokio::net::windows::named_pipe::ClientOptions;

fn benchmark_json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialization");

    group.bench_function("request", |b| {
        b.iter(|| {
            let request = json!({
                "jsonrpc": "2.0",
                "method": "daemon/ping",
                "id": 1
            });
            black_box(
                serde_json::to_string(&request).expect("serialize JSON-RPC request to string"),
            );
        });
    });

    group.bench_function("response", |b| {
        b.iter(|| {
            let response = json!({
                "jsonrpc": "2.0",
                "result": {"status": "pong"},
                "id": 1
            });
            black_box(
                serde_json::to_string(&response).expect("serialize JSON-RPC response to string"),
            );
        });
    });

    group.bench_function("complex_message", |b| {
        b.iter(|| {
            let message = json!({
                "jsonrpc": "2.0",
                "method": "daemon/broadcast",
                "params": {
                    "event_type": "terminal.output",
                    "data": {
                        "terminal_id": "term-001",
                        "content": "Some terminal output",
                        "timestamp": 1234567890
                    }
                },
                "id": 1
            });
            black_box(
                serde_json::to_string(&message)
                    .expect("serialize complex JSON-RPC message to string"),
            );
        });
    });

    group.finish();
}

#[cfg(windows)]
fn benchmark_round_trip_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("round_trip_latency");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(100);

    // Only run if daemon is available
    let rt = Runtime::new().expect("create tokio runtime to probe for daemon availability");
    let pipe_name = r"\\.\pipe\wezterm-utils-test";

    // Test if daemon is running
    let daemon_available = rt.block_on(async { ClientOptions::new().open(pipe_name).is_ok() });

    if !daemon_available {
        println!("Skipping round-trip benchmark: daemon not running");
        group.finish();
        return;
    }

    group.bench_function("ping", |b| {
        let rt = Runtime::new().expect("create tokio runtime for ping round-trip benchmark");

        b.to_async(&rt).iter(|| async {
            let client = ClientOptions::new()
                .open(pipe_name)
                .expect("Failed to connect");

            let (reader, mut writer) = tokio::io::split(client);
            let mut reader = BufReader::new(reader);

            let request = json!({
                "jsonrpc": "2.0",
                "method": "daemon/ping",
                "id": 1
            });

            let json = format!(
                "{}\n",
                serde_json::to_string(&request).expect("serialize ping request to JSON string")
            );
            writer
                .write_all(json.as_bytes())
                .await
                .expect("write ping request to named pipe");
            writer
                .flush()
                .await
                .expect("flush ping request to named pipe");

            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .expect("read ping response from named pipe");

            black_box(
                serde_json::from_str::<serde_json::Value>(&line).expect("parse ping response JSON"),
            );
        });
    });

    group.finish();
}

#[cfg(not(windows))]
fn benchmark_round_trip_latency(_c: &mut Criterion) {
    println!("Round-trip benchmarks require Windows");
}

fn benchmark_protocol_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("protocol_parsing");

    let request_json = r#"{"jsonrpc":"2.0","method":"daemon/ping","id":1}"#;
    let response_json = r#"{"jsonrpc":"2.0","result":{"status":"pong"},"id":1}"#;

    group.bench_function("parse_request", |b| {
        b.iter(|| {
            black_box(
                serde_json::from_str::<serde_json::Value>(request_json)
                    .expect("parse JSON-RPC request from string"),
            );
        });
    });

    group.bench_function("parse_response", |b| {
        b.iter(|| {
            black_box(
                serde_json::from_str::<serde_json::Value>(response_json)
                    .expect("parse JSON-RPC response from string"),
            );
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_json_serialization,
    benchmark_protocol_parsing,
    benchmark_round_trip_latency,
);

criterion_main!(benches);
