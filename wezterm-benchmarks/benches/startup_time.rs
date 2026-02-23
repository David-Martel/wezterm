use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;
use std::time::{Duration, Instant};
use std::process::Command;
use wezterm_benchmarks::startup::{
    LazyInitializer, PreloadedResources, StartupOptimizer,
    DeferredInitializer
};

fn bench_utility_startup(c: &mut Criterion) {
    let mut group = c.benchmark_group("utility_startup");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    // Test different utility startup times
    let utilities = vec![
        ("wedit", "wedit --version"),
        ("explorer", "wezterm-explorer --version"),
        ("daemon", "wezterm-utils-daemon --version"),
    ];

    for (name, cmd) in utilities {
        group.bench_function(name, |b| {
            b.iter(|| {
                let start = Instant::now();
                let output = Command::new("cmd")
                    .args(["/C", cmd])
                    .output()
                    .unwrap();
                let duration = start.elapsed();
                black_box((output, duration))
            });
        });
    }

    group.finish();
}

fn bench_initialization_strategies(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("initialization_strategies");

    // Simulate expensive initialization tasks
    fn expensive_init() -> Vec<u8> {
        std::thread::sleep(Duration::from_millis(10));
        vec![0u8; 1_000_000]
    }

    fn load_config() -> serde_json::Value {
        std::thread::sleep(Duration::from_millis(5));
        serde_json::json!({
            "setting1": "value1",
            "setting2": 42,
            "setting3": [1, 2, 3]
        })
    }

    fn connect_to_service() -> String {
        std::thread::sleep(Duration::from_millis(15));
        "connected".to_string()
    }

    group.bench_function("eager_initialization", |b| {
        b.iter(|| {
            let start = Instant::now();

            // Everything initialized upfront
            let data = expensive_init();
            let config = load_config();
            let connection = connect_to_service();

            let app = AppState {
                data,
                config,
                connection,
            };

            let duration = start.elapsed();
            black_box((app, duration))
        });
    });

    group.bench_function("lazy_initialization", |b| {
        b.iter(|| {
            let start = Instant::now();

            // Use lazy initialization
            let app = LazyAppState::new();

            // First access triggers initialization
            let _ = app.get_data();

            let duration = start.elapsed();
            black_box((app, duration))
        });
    });

    group.bench_function("deferred_initialization", |b| {
        b.to_async(&rt).iter(|| async {
            let start = Instant::now();

            // Initialize only critical components
            let app = DeferredInitializer::new();

            // Start app immediately
            app.start().await;

            // Non-critical components initialized in background
            app.wait_for_full_init().await;

            let duration = start.elapsed();
            black_box((app, duration))
        });
    });

    group.bench_function("preloaded_resources", |b| {
        // Preload resources once
        let resources = PreloadedResources::load();

        b.iter(|| {
            let start = Instant::now();

            // Use preloaded resources for fast startup
            let app = AppWithPreloaded::new(&resources);

            let duration = start.elapsed();
            black_box((app, duration))
        });
    });

    group.finish();
}

fn bench_dependency_loading(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("dependency_loading");

    group.bench_function("sequential_loading", |b| {
        b.iter(|| {
            let start = Instant::now();

            // Load dependencies one by one
            let dep1 = load_dependency("dep1");
            let dep2 = load_dependency("dep2");
            let dep3 = load_dependency("dep3");
            let dep4 = load_dependency("dep4");

            let duration = start.elapsed();
            black_box((dep1, dep2, dep3, dep4, duration))
        });
    });

    group.bench_function("parallel_loading", |b| {
        b.to_async(&rt).iter(|| async {
            let start = Instant::now();

            // Load dependencies in parallel
            let (dep1, dep2, dep3, dep4) = tokio::join!(
                async_load_dependency("dep1"),
                async_load_dependency("dep2"),
                async_load_dependency("dep3"),
                async_load_dependency("dep4")
            );

            let duration = start.elapsed();
            black_box((dep1, dep2, dep3, dep4, duration))
        });
    });

    group.bench_function("cached_loading", |b| {
        let cache = DependencyCache::new();

        b.iter(|| {
            let start = Instant::now();

            // Use cached dependencies
            let dep1 = cache.get_or_load("dep1");
            let dep2 = cache.get_or_load("dep2");
            let dep3 = cache.get_or_load("dep3");
            let dep4 = cache.get_or_load("dep4");

            let duration = start.elapsed();
            black_box((dep1, dep2, dep3, dep4, duration))
        });
    });

    group.finish();
}

fn bench_config_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_parsing");

    let json_config = serde_json::json!({
        "utilities": {
            "wedit": {
                "theme": "dark",
                "font_size": 12,
                "plugins": ["syntax", "autocomplete", "git"]
            },
            "explorer": {
                "show_hidden": true,
                "sort_by": "name",
                "view_mode": "details"
            }
        },
        "ipc": {
            "timeout": 5000,
            "retry_count": 3,
            "buffer_size": 4096
        }
    }).to_string();

    let toml_config = r#"
        [utilities.wedit]
        theme = "dark"
        font_size = 12
        plugins = ["syntax", "autocomplete", "git"]

        [utilities.explorer]
        show_hidden = true
        sort_by = "name"
        view_mode = "details"

        [ipc]
        timeout = 5000
        retry_count = 3
        buffer_size = 4096
    "#;

    group.bench_function("json_parsing", |b| {
        b.iter(|| {
            let parsed: serde_json::Value = serde_json::from_str(&json_config).unwrap();
            black_box(parsed)
        });
    });

    group.bench_function("toml_parsing", |b| {
        b.iter(|| {
            let parsed: toml::Value = toml::from_str(toml_config).unwrap();
            black_box(parsed)
        });
    });

    group.bench_function("cached_config", |b| {
        let cache = ConfigCache::new();
        cache.set("config", json_config.clone());

        b.iter(|| {
            let config = cache.get("config").unwrap();
            black_box(config)
        });
    });

    group.finish();
}

fn bench_cold_vs_warm_start(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_vs_warm_start");
    group.sample_size(10);

    group.bench_function("cold_start", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;

            for _ in 0..iters {
                // Clear any caches/state
                clear_system_caches();

                let start = Instant::now();
                let _app = ColdStartApp::new();
                total += start.elapsed();
            }

            total
        });
    });

    group.bench_function("warm_start", |b| {
        // Warm up the system
        let _ = WarmStartApp::new();
        clear_app_state();

        b.iter(|| {
            let start = Instant::now();
            let _app = WarmStartApp::new();
            start.elapsed()
        });
    });

    group.finish();
}

// Helper structures and functions
struct AppState {
    data: Vec<u8>,
    config: serde_json::Value,
    connection: String,
}

struct LazyAppState {
    data: once_cell::sync::Lazy<Vec<u8>>,
    config: once_cell::sync::Lazy<serde_json::Value>,
    connection: once_cell::sync::Lazy<String>,
}

impl LazyAppState {
    fn new() -> Self {
        Self {
            data: once_cell::sync::Lazy::new(|| vec![0u8; 1_000_000]),
            config: once_cell::sync::Lazy::new(|| serde_json::json!({})),
            connection: once_cell::sync::Lazy::new(|| "connected".to_string()),
        }
    }

    fn get_data(&self) -> &[u8] {
        &self.data
    }
}

struct AppWithPreloaded {
    resources: PreloadedResources,
}

impl AppWithPreloaded {
    fn new(resources: &PreloadedResources) -> Self {
        Self {
            resources: resources.clone(),
        }
    }
}

struct DependencyCache {
    cache: dashmap::DashMap<String, Vec<u8>>,
}

impl DependencyCache {
    fn new() -> Self {
        Self {
            cache: dashmap::DashMap::new(),
        }
    }

    fn get_or_load(&self, name: &str) -> Vec<u8> {
        self.cache
            .entry(name.to_string())
            .or_insert_with(|| load_dependency(name))
            .clone()
    }
}

fn load_dependency(name: &str) -> Vec<u8> {
    std::thread::sleep(Duration::from_millis(5));
    vec![0u8; 1000]
}

async fn async_load_dependency(name: &str) -> Vec<u8> {
    tokio::time::sleep(Duration::from_millis(5)).await;
    vec![0u8; 1000]
}

struct ConfigCache {
    cache: dashmap::DashMap<String, String>,
}

impl ConfigCache {
    fn new() -> Self {
        Self {
            cache: dashmap::DashMap::new(),
        }
    }

    fn set(&self, key: &str, value: String) {
        self.cache.insert(key.to_string(), value);
    }

    fn get(&self, key: &str) -> Option<String> {
        self.cache.get(key).map(|v| v.clone())
    }
}

struct ColdStartApp;
struct WarmStartApp;

impl ColdStartApp {
    fn new() -> Self {
        std::thread::sleep(Duration::from_millis(50));
        Self
    }
}

impl WarmStartApp {
    fn new() -> Self {
        std::thread::sleep(Duration::from_millis(10));
        Self
    }
}

fn clear_system_caches() {
    // Simulate clearing system caches
    std::thread::sleep(Duration::from_millis(10));
}

fn clear_app_state() {
    // Simulate clearing app state
    std::thread::sleep(Duration::from_millis(5));
}

criterion_group!(
    benches,
    bench_utility_startup,
    bench_initialization_strategies,
    bench_dependency_loading,
    bench_config_parsing,
    bench_cold_vs_warm_start
);
criterion_main!(benches);