//! Real-time performance monitoring dashboard for WezTerm utilities

use clap::Parser;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use wezterm_benchmarks::monitoring::{AlertSystem, MetricsCollector, PerfMonitor, ReportGenerator};

#[derive(Parser)]
#[clap(name = "perf-monitor")]
#[clap(about = "Real-time performance monitoring for WezTerm utilities")]
struct Args {
    /// HTTP port for metrics endpoint
    #[clap(short, long, default_value = "9090")]
    port: u16,

    /// Update interval in seconds
    #[clap(short, long, default_value = "1")]
    interval: u64,

    /// Enable console output
    #[clap(short, long)]
    console: bool,

    /// Export metrics in Prometheus format
    #[clap(short = 'p', long)]
    prometheus: bool,

    /// Generate performance report every N seconds
    #[clap(short, long)]
    report_interval: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize monitoring components
    let collector = Arc::new(MetricsCollector::new());
    let monitor = Arc::new(PerfMonitor::new(collector.clone()));
    let alert_system = Arc::new(AlertSystem::new());
    let report_generator = Arc::new(ReportGenerator::new());

    // Subscribe to alerts
    let mut alert_rx = alert_system.subscribe().await;

    // Start alert handler
    tokio::spawn(async move {
        while let Some(alert) = alert_rx.recv().await {
            println!(
                "⚠️  ALERT [{}] {}: {}",
                match alert.severity {
                    wezterm_benchmarks::monitoring::AlertSeverity::Info => "INFO",
                    wezterm_benchmarks::monitoring::AlertSeverity::Warning => "WARN",
                    wezterm_benchmarks::monitoring::AlertSeverity::Critical => "CRIT",
                },
                alert.component,
                alert.message
            );
        }
    });

    // Start HTTP server for metrics
    if args.prometheus {
        let collector = collector.clone();
        tokio::spawn(async move {
            use warp::Filter;

            let metrics = warp::path("metrics").map(move || collector.export_metrics());

            println!(
                "📊 Prometheus metrics available at http://localhost:{}/metrics",
                args.port
            );
            warp::serve(metrics).run(([0, 0, 0, 0], args.port)).await;
        });
    }

    // Start report generation
    if let Some(report_interval) = args.report_interval {
        let report_gen = report_generator.clone();
        let monitor = monitor.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(report_interval));

            loop {
                interval.tick().await;

                let report = report_gen.generate_report();

                println!("\n📈 PERFORMANCE REPORT");
                println!("═══════════════════════════════════════════");
                println!("Duration: {:?}", report.duration);
                println!("\nIPC Latency:");
                println!("  Average: {:.2}ms", report.ipc_latency_avg);
                println!("  P50: {:.2}ms", report.ipc_latency_p50);
                println!("  P99: {:.2}ms", report.ipc_latency_p99);
                println!("\nMemory Usage:");
                println!("  Average: {:.2}MB", report.memory_usage_avg);
                println!("  Maximum: {:.2}MB", report.memory_usage_max);
                println!("\nCPU Usage:");
                println!("  Average: {:.2}%", report.cpu_usage_avg);
                println!("  Maximum: {:.2}%", report.cpu_usage_max);
                println!("═══════════════════════════════════════════\n");
            }
        });
    }

    // Main monitoring loop
    let mut update_interval = interval(Duration::from_secs(args.interval));

    println!("🚀 WezTerm Performance Monitor Started");
    println!("═══════════════════════════════════════════");

    loop {
        update_interval.tick().await;

        let metrics = monitor.get_metrics();

        // Record metrics for reporting
        report_generator.record(metrics.clone());

        // Check for alerts
        alert_system.check_metrics(&metrics);

        // Console output if enabled
        if args.console {
            print_metrics(&metrics);
        }
    }
}

fn print_metrics(metrics: &wezterm_benchmarks::monitoring::PerformanceMetrics) {
    // Clear screen and move cursor to top (works on most terminals)
    print!("\x1B[2J\x1B[1;1H");

    println!("╔═══════════════════════════════════════════╗");
    println!("║     WezTerm Utilities Performance         ║");
    println!("╠═══════════════════════════════════════════╣");

    println!("║ 📡 IPC Performance                        ║");
    println!(
        "║   Latency P50: {:>8.2} ms               ║",
        metrics.ipc_latency_p50
    );
    println!(
        "║   Latency P99: {:>8.2} ms               ║",
        metrics.ipc_latency_p99
    );
    println!(
        "║   Active Connections: {:>3}                ║",
        metrics.active_connections
    );

    println!("╠═══════════════════════════════════════════╣");

    println!("║ 💾 Resource Usage                         ║");
    println!(
        "║   Memory: {:>8.2} MB                    ║",
        metrics.memory_usage_mb
    );
    println!(
        "║   CPU:    {:>8.2} %                     ║",
        metrics.cpu_usage_percent
    );

    println!("╠═══════════════════════════════════════════╣");

    println!("║ 📁 File Operations                        ║");
    println!(
        "║   Ops/sec: {:>8.2}                      ║",
        metrics.file_ops_per_sec
    );
    println!(
        "║   Git Cache Hit Rate: {:>5.1}%            ║",
        metrics.git_cache_hit_rate * 100.0
    );

    println!("╠═══════════════════════════════════════════╣");

    println!("║ ⚡ Startup                                ║");
    println!(
        "║   Average Time: {:>8.2} ms              ║",
        metrics.startup_time_avg
    );

    println!("╚═══════════════════════════════════════════╝");

    // Performance indicators
    println!("\n Performance Indicators:");

    if metrics.ipc_latency_p99 < 50.0 {
        println!(" ✅ IPC Latency: Excellent");
    } else if metrics.ipc_latency_p99 < 100.0 {
        println!(" ⚠️  IPC Latency: Acceptable");
    } else {
        println!(" ❌ IPC Latency: Poor");
    }

    if metrics.memory_usage_mb < 50.0 {
        println!(" ✅ Memory Usage: Excellent");
    } else if metrics.memory_usage_mb < 100.0 {
        println!(" ⚠️  Memory Usage: Acceptable");
    } else {
        println!(" ❌ Memory Usage: High");
    }

    if metrics.cpu_usage_percent < 30.0 {
        println!(" ✅ CPU Usage: Excellent");
    } else if metrics.cpu_usage_percent < 60.0 {
        println!(" ⚠️  CPU Usage: Acceptable");
    } else {
        println!(" ❌ CPU Usage: High");
    }

    if metrics.startup_time_avg < 200.0 {
        println!(" ✅ Startup Time: Excellent");
    } else if metrics.startup_time_avg < 500.0 {
        println!(" ⚠️  Startup Time: Acceptable");
    } else {
        println!(" ❌ Startup Time: Slow");
    }
}
