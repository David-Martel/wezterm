//! Stress testing tool for WezTerm utilities

use clap::Parser;
use futures::future::join_all;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;
use wezterm_benchmarks::{
    fs::{DirectoryScanner, ParallelScanner},
    git::GitStatusCache,
    ipc::ConnectionPool,
    memory::{BufferPool, MemoryTracker},
};

#[derive(Parser)]
#[clap(name = "stress-test")]
#[clap(about = "Stress testing tool for WezTerm utilities")]
struct Args {
    /// Test duration in seconds
    #[clap(short = 'd', long, default_value = "60")]
    duration: u64,

    /// Number of concurrent clients
    #[clap(short = 'c', long, default_value = "10")]
    clients: usize,

    /// Operations per second per client
    #[clap(short = 'o', long, default_value = "100")]
    ops_per_sec: u64,

    /// Test mode (ipc, file, git, memory, all)
    #[clap(short = 'm', long, default_value = "all")]
    mode: String,

    /// Enable memory leak detection
    #[clap(short = 'l', long)]
    leak_detection: bool,

    /// Verbose output
    #[clap(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("🔥 WezTerm Utilities Stress Test");
    println!("═══════════════════════════════════════════");
    println!("Duration: {} seconds", args.duration);
    println!("Concurrent Clients: {}", args.clients);
    println!("Operations/sec/client: {}", args.ops_per_sec);
    println!("Test Mode: {}", args.mode);
    println!("═══════════════════════════════════════════\n");

    let start = Instant::now();
    let test_duration = Duration::from_secs(args.duration);

    // Initialize memory tracking if enabled
    let memory_tracker = if args.leak_detection {
        Some(Arc::new(MemoryTracker::new()))
    } else {
        None
    };

    // Run selected tests
    match args.mode.as_str() {
        "ipc" => run_ipc_stress_test(&args, test_duration).await?,
        "file" => run_file_stress_test(&args, test_duration).await?,
        "git" => run_git_stress_test(&args, test_duration).await?,
        "memory" => run_memory_stress_test(&args, test_duration).await?,
        "all" => run_all_tests(&args, test_duration).await?,
        _ => {
            eprintln!("Unknown test mode: {}", args.mode);
            std::process::exit(1);
        }
    }

    let elapsed = start.elapsed();

    // Check for memory leaks
    if let Some(tracker) = memory_tracker {
        if tracker.check_for_leak() {
            println!("\n⚠️  WARNING: Potential memory leak detected!");

            let leaks = tracker.get_leaked_allocations();
            for (location, size, age) in leaks.iter().take(10) {
                println!("  {} - {} bytes, age: {:?}", location, size, age);
            }
        } else {
            println!("\n✅ No memory leaks detected");
        }
    }

    // Print summary
    println!("\n📊 Test Summary");
    println!("═══════════════════════════════════════════");
    println!("Total Duration: {:?}", elapsed);
    println!("Status: COMPLETED");

    Ok(())
}

async fn run_ipc_stress_test(
    args: &Args,
    duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting IPC stress test...");

    let pool = Arc::new(ConnectionPool::new(args.clients * 2).await);
    let start = Instant::now();
    let total_ops = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let errors = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let tasks: Vec<_> = (0..args.clients)
        .map(|client_id| {
            let pool = pool.clone();
            let total_ops = total_ops.clone();
            let errors = errors.clone();
            let ops_per_sec = args.ops_per_sec;

            tokio::spawn(async move {
                let mut interval = interval(Duration::from_millis(1000 / ops_per_sec));
                let client_start = Instant::now();

                while client_start.elapsed() < duration {
                    interval.tick().await;

                    let client = pool.get_or_create(&format!("client_{}", client_id)).await;

                    match client
                        .send_request::<_, String>("echo", &"test_payload")
                        .await
                    {
                        Ok(_) => {
                            total_ops.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                        Err(_) => {
                            errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
            })
        })
        .collect();

    // Monitor progress
    let monitor_task = {
        let total_ops = total_ops.clone();
        let errors = errors.clone();
        let verbose = args.verbose;

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));

            while start.elapsed() < duration {
                interval.tick().await;

                if verbose {
                    let ops = total_ops.load(std::sync::atomic::Ordering::Relaxed);
                    let err = errors.load(std::sync::atomic::Ordering::Relaxed);
                    let elapsed = start.elapsed().as_secs();
                    let rate = if elapsed > 0 {
                        ops / elapsed as usize
                    } else {
                        0
                    };

                    println!(
                        "  [{:>3}s] Operations: {}, Errors: {}, Rate: {}/s",
                        elapsed, ops, err, rate
                    );
                }
            }
        })
    };

    // Wait for all tasks
    join_all(tasks).await;
    monitor_task.abort();

    let total = total_ops.load(std::sync::atomic::Ordering::Relaxed);
    let err = errors.load(std::sync::atomic::Ordering::Relaxed);
    let success_rate = if total > 0 {
        ((total - err) as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    println!("\nIPC Test Results:");
    println!("  Total Operations: {}", total);
    println!("  Successful: {}", total - err);
    println!("  Failed: {}", err);
    println!("  Success Rate: {:.2}%", success_rate);
    println!("  Average Rate: {}/s", total / duration.as_secs() as usize);

    Ok(())
}

async fn run_file_stress_test(
    args: &Args,
    duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting file system stress test...");

    use tempfile::TempDir;

    // Create test directory structure
    let temp_dir = TempDir::new()?;
    for i in 0..100 {
        let dir = temp_dir.path().join(format!("dir_{}", i));
        std::fs::create_dir_all(&dir)?;

        for j in 0..10 {
            let file = dir.join(format!("file_{}.txt", j));
            std::fs::write(&file, format!("Content {}-{}", i, j))?;
        }
    }

    let scanner = Arc::new(DirectoryScanner::new());
    let parallel_scanner = Arc::new(ParallelScanner::new());
    let start = Instant::now();
    let mut total_scans = 0;

    while start.elapsed() < duration {
        // Regular scan
        let _ = scanner.scan(temp_dir.path()).await?;
        total_scans += 1;

        // Parallel scan
        let _ = parallel_scanner.scan(temp_dir.path()).await?;
        total_scans += 1;

        // Cached scan
        let _ = scanner.scan_cached(temp_dir.path());
        total_scans += 1;

        if args.verbose && total_scans % 100 == 0 {
            println!("  Completed {} scans", total_scans);
        }
    }

    println!("\nFile System Test Results:");
    println!("  Total Scans: {}", total_scans);
    println!(
        "  Average Rate: {}/s",
        total_scans / duration.as_secs() as usize
    );

    Ok(())
}

async fn run_git_stress_test(
    args: &Args,
    duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Git operations stress test...");

    use git2::{Repository, Signature};
    use tempfile::TempDir;

    // Create test repo
    let temp_dir = TempDir::new()?;
    let repo = Repository::init(temp_dir.path())?;

    // Create initial commit
    let sig = Signature::now("Test", "test@example.com")?;
    for i in 0..50 {
        std::fs::write(
            temp_dir.path().join(format!("file_{}.txt", i)),
            format!("content {}", i),
        )?;
    }

    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

    let cache = Arc::new(GitStatusCache::new(Duration::from_secs(1)));
    let start = Instant::now();
    let mut total_ops = 0;
    let _cache_hits = 0;

    while start.elapsed() < duration {
        // Status check
        let _ = cache.get_status(temp_dir.path())?;
        total_ops += 1;

        // Diff check
        let _ = cache.get_diff(temp_dir.path())?;
        total_ops += 1;

        // Log check
        let _ = cache.get_log(temp_dir.path(), 10)?;
        total_ops += 1;

        if args.verbose && total_ops % 100 == 0 {
            println!("  Completed {} Git operations", total_ops);
        }
    }

    println!("\nGit Test Results:");
    println!("  Total Operations: {}", total_ops);
    println!(
        "  Average Rate: {}/s",
        total_ops / duration.as_secs() as usize
    );

    Ok(())
}

async fn run_memory_stress_test(
    args: &Args,
    duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting memory stress test...");

    let buffer_pool = Arc::new(BufferPool::new(1000, 4096));
    let start = Instant::now();
    let mut total_allocations = 0;

    while start.elapsed() < duration {
        // Allocate and release buffers rapidly
        let mut buffers = Vec::new();

        for _ in 0..100 {
            let buf = buffer_pool.acquire();
            buffers.push(buf);
            total_allocations += 1;
        }

        // Simulate work
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Buffers automatically returned to pool on drop
        drop(buffers);

        if args.verbose && total_allocations % 10000 == 0 {
            println!("  Completed {} allocations", total_allocations);
        }
    }

    println!("\nMemory Test Results:");
    println!("  Total Allocations: {}", total_allocations);
    println!(
        "  Average Rate: {}/s",
        total_allocations / duration.as_secs() as usize
    );

    Ok(())
}

async fn run_all_tests(args: &Args, duration: Duration) -> Result<(), Box<dyn std::error::Error>> {
    let test_duration = Duration::from_secs(duration.as_secs() / 4);

    println!("Running all tests sequentially...\n");

    run_ipc_stress_test(args, test_duration).await?;
    println!();

    run_file_stress_test(args, test_duration).await?;
    println!();

    run_git_stress_test(args, test_duration).await?;
    println!();

    run_memory_stress_test(args, test_duration).await?;

    Ok(())
}
