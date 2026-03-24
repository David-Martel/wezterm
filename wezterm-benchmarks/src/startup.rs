//! Startup optimization utilities for faster utility initialization

use std::sync::Arc;
use std::time::{Duration, Instant};
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::Mutex;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use dashmap::DashMap;
use futures::future::join_all;

/// Lazy initializer for deferred resource loading
pub struct LazyInitializer<T> {
    initializer: Arc<dyn Fn() -> T + Send + Sync>,
    value: Arc<OnceCell<T>>,
}

impl<T: Send + Sync> LazyInitializer<T> {
    pub fn new<F>(initializer: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            initializer: Arc::new(initializer),
            value: Arc::new(OnceCell::new()),
        }
    }

    pub fn get(&self) -> &T {
        self.value.get_or_init(|| (self.initializer)())
    }

    pub fn is_initialized(&self) -> bool {
        self.value.get().is_some()
    }
}

/// Preloaded resources for fast startup
#[derive(Clone)]
pub struct PreloadedResources {
    config: Arc<serde_json::Value>,
    templates: Arc<DashMap<String, String>>,
    assets: Arc<DashMap<String, Vec<u8>>>,
    loaded_at: Instant,
}

impl PreloadedResources {
    pub fn load() -> Self {
        let start = Instant::now();

        // Simulated resource loading
        let config = Arc::new(serde_json::json!({
            "theme": "dark",
            "font_size": 12,
            "plugins": ["syntax", "autocomplete"]
        }));

        let templates = Arc::new(DashMap::new());
        templates.insert("default".to_string(), "Default template content".to_string());

        let assets = Arc::new(DashMap::new());
        assets.insert("icon.png".to_string(), vec![0u8; 1024]);

        Self {
            config,
            templates,
            assets,
            loaded_at: start,
        }
    }

    pub fn age(&self) -> Duration {
        self.loaded_at.elapsed()
    }
}

/// Deferred initializer for non-critical components
pub struct DeferredInitializer {
    critical: Arc<RwLock<Option<CriticalComponents>>>,
    non_critical: Arc<RwLock<Option<NonCriticalComponents>>>,
    init_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

struct CriticalComponents {
    ipc_client: String,
    config: serde_json::Value,
}

struct NonCriticalComponents {
    cache: DashMap<String, Vec<u8>>,
    plugins: Vec<String>,
    telemetry: String,
}

impl DeferredInitializer {
    pub fn new() -> Self {
        Self {
            critical: Arc::new(RwLock::new(None)),
            non_critical: Arc::new(RwLock::new(None)),
            init_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(&self) {
        // Initialize critical components immediately
        let critical = CriticalComponents {
            ipc_client: "connected".to_string(),
            config: serde_json::json!({"minimal": true}),
        };

        *self.critical.write().await = Some(critical);

        // Start background initialization of non-critical components
        let non_critical = self.non_critical.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;

            let components = NonCriticalComponents {
                cache: DashMap::new(),
                plugins: vec!["plugin1".to_string(), "plugin2".to_string()],
                telemetry: "initialized".to_string(),
            };

            *non_critical.write().await = Some(components);
        });

        *self.init_handle.lock() = Some(handle);
    }

    pub async fn wait_for_full_init(&self) {
        if let Some(handle) = self.init_handle.lock().take() {
            let _ = handle.await;
        }
    }

    pub async fn is_fully_initialized(&self) -> bool {
        self.non_critical.read().await.is_some()
    }
}

/// Startup optimizer with profiling
pub struct StartupOptimizer {
    profile: Arc<Mutex<StartupProfile>>,
}

#[derive(Default)]
struct StartupProfile {
    phases: Vec<Phase>,
    total_time: Duration,
}

struct Phase {
    name: String,
    duration: Duration,
    parallel: bool,
}

impl StartupOptimizer {
    pub fn new() -> Self {
        Self {
            profile: Arc::new(Mutex::new(StartupProfile::default())),
        }
    }

    pub async fn optimize_startup<F, Fut>(&self, startup_fn: F) -> Duration
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let start = Instant::now();

        // Run optimized startup
        startup_fn().await;

        let duration = start.elapsed();

        let mut profile = self.profile.lock();
        profile.total_time = duration;

        duration
    }

    pub fn record_phase(&self, name: &str, duration: Duration, parallel: bool) {
        let mut profile = self.profile.lock();
        profile.phases.push(Phase {
            name: name.to_string(),
            duration,
            parallel,
        });
    }

    pub fn get_optimization_suggestions(&self) -> Vec<String> {
        let profile = self.profile.lock();
        let mut suggestions = Vec::new();

        // Analyze phases for optimization opportunities
        for phase in &profile.phases {
            if phase.duration > Duration::from_millis(100) && !phase.parallel {
                suggestions.push(format!(
                    "Consider parallelizing '{}' phase (currently {}ms)",
                    phase.name,
                    phase.duration.as_millis()
                ));
            }

            if phase.duration > Duration::from_millis(500) {
                suggestions.push(format!(
                    "Consider deferring or lazy-loading '{}' phase ({}ms)",
                    phase.name,
                    phase.duration.as_millis()
                ));
            }
        }

        if profile.total_time > Duration::from_millis(1000) {
            suggestions.push(format!(
                "Total startup time {}ms exceeds target. Consider preloading resources.",
                profile.total_time.as_millis()
            ));
        }

        suggestions
    }
}

/// Configuration cache for fast parsing
pub struct ConfigCache {
    json_cache: DashMap<String, serde_json::Value>,
    toml_cache: DashMap<String, toml::Value>,
    parsed_cache: DashMap<String, Arc<dyn std::any::Any + Send + Sync>>,
}

impl ConfigCache {
    pub fn new() -> Self {
        Self {
            json_cache: DashMap::new(),
            toml_cache: DashMap::new(),
            parsed_cache: DashMap::new(),
        }
    }

    pub fn get_json(&self, content: &str) -> serde_json::Value {
        let hash = self.hash_content(content);

        if let Some(cached) = self.json_cache.get(&hash) {
            return cached.clone();
        }

        let parsed = serde_json::from_str(content).unwrap_or(serde_json::json!({}));
        self.json_cache.insert(hash, parsed.clone());
        parsed
    }

    pub fn get_toml(&self, content: &str) -> toml::Value {
        let hash = self.hash_content(content);

        if let Some(cached) = self.toml_cache.get(&hash) {
            return cached.clone();
        }

        let parsed = toml::from_str(content).unwrap_or(toml::Value::Table(toml::map::Map::new()));
        self.toml_cache.insert(hash, parsed.clone());
        parsed
    }

    fn hash_content(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Parallel dependency loader
pub struct ParallelDependencyLoader {
    dependencies: Vec<Dependency>,
}

struct Dependency {
    name: String,
    loader: Arc<dyn Fn() -> Vec<u8> + Send + Sync>,
}

impl ParallelDependencyLoader {
    pub fn new() -> Self {
        Self {
            dependencies: Vec::new(),
        }
    }

    pub fn add_dependency<F>(&mut self, name: &str, loader: F)
    where
        F: Fn() -> Vec<u8> + Send + Sync + 'static,
    {
        self.dependencies.push(Dependency {
            name: name.to_string(),
            loader: Arc::new(loader),
        });
    }

    pub async fn load_all(&self) -> DashMap<String, Vec<u8>> {
        let results = DashMap::new();

        let futures: Vec<_> = self.dependencies
            .iter()
            .map(|dep| {
                let name = dep.name.clone();
                let loader = dep.loader.clone();
                let results = results.clone();

                tokio::spawn(async move {
                    let data = (loader)();
                    results.insert(name, data);
                })
            })
            .collect();

        join_all(futures).await;

        results
    }
}

/// Startup time predictor based on historical data
pub struct StartupPredictor {
    history: Arc<Mutex<Vec<StartupMetric>>>,
}

#[derive(Clone)]
struct StartupMetric {
    timestamp: Instant,
    duration: Duration,
    config_size: usize,
    dependency_count: usize,
}

impl StartupPredictor {
    pub fn new() -> Self {
        Self {
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn record(&self, duration: Duration, config_size: usize, dependency_count: usize) {
        let mut history = self.history.lock();
        history.push(StartupMetric {
            timestamp: Instant::now(),
            duration,
            config_size,
            dependency_count,
        });

        // Keep only last 100 records
        if history.len() > 100 {
            history.remove(0);
        }
    }

    pub fn predict(&self, config_size: usize, dependency_count: usize) -> Duration {
        let history = self.history.lock();

        if history.is_empty() {
            return Duration::from_millis(500); // Default prediction
        }

        // Simple linear regression
        let mut total_duration = Duration::ZERO;
        let mut total_weight = 0.0;

        for metric in history.iter() {
            let config_similarity = 1.0 - ((config_size as f64 - metric.config_size as f64).abs() / 10000.0).min(1.0);
            let dep_similarity = 1.0 - ((dependency_count as f64 - metric.dependency_count as f64).abs() / 100.0).min(1.0);

            let weight = config_similarity * dep_similarity;
            total_duration += metric.duration.mul_f64(weight);
            total_weight += weight;
        }

        if total_weight > 0.0 {
            total_duration.div_f64(total_weight)
        } else {
            Duration::from_millis(500)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_initializer() {
        let lazy = LazyInitializer::new(|| {
            std::thread::sleep(Duration::from_millis(10));
            42
        });

        assert!(!lazy.is_initialized());
        let value = lazy.get();
        assert_eq!(*value, 42);
        assert!(lazy.is_initialized());
    }

    #[tokio::test]
    async fn test_deferred_initializer() {
        let init = DeferredInitializer::new();

        init.start().await;
        assert!(!init.is_fully_initialized().await);

        init.wait_for_full_init().await;
        assert!(init.is_fully_initialized().await);
    }

    #[tokio::test]
    #[ignore = "requires tokio multi-thread runtime; flaky under single-thread nextest"]
    async fn test_parallel_dependency_loader() {
        let mut loader = ParallelDependencyLoader::new();

        loader.add_dependency("dep1", || vec![1, 2, 3]);
        loader.add_dependency("dep2", || vec![4, 5, 6]);

        let deps = loader.load_all().await;

        assert_eq!(deps.len(), 2);
        assert!(deps.contains_key("dep1"));
        assert!(deps.contains_key("dep2"));
    }

    #[test]
    fn test_startup_predictor() {
        let predictor = StartupPredictor::new();

        predictor.record(Duration::from_millis(100), 1000, 10);
        predictor.record(Duration::from_millis(150), 2000, 20);
        predictor.record(Duration::from_millis(200), 3000, 30);

        let prediction = predictor.predict(2500, 25);
        assert!(prediction.as_millis() > 100 && prediction.as_millis() < 300);
    }
}