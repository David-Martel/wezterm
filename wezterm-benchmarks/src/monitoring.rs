//! Performance monitoring and metrics collection

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use parking_lot::RwLock;
use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec,
    Encoder, TextEncoder, Opts, Registry,
};
use serde::{Serialize, Deserialize};
use systemstat::{System, Platform};
use tokio::sync::mpsc;
use dashmap::DashMap;

/// Performance metrics collector
pub struct MetricsCollector {
    registry: Arc<Registry>,
    ipc_latency: Arc<HistogramVec>,
    file_ops: Arc<CounterVec>,
    memory_usage: Arc<GaugeVec>,
    cpu_usage: Arc<GaugeVec>,
    startup_time: Arc<Histogram>,
    git_cache_hits: Arc<Counter>,
    git_cache_misses: Arc<Counter>,
    active_connections: Arc<Gauge>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Registry::new();

        let ipc_latency = HistogramVec::new(
            HistogramOpts::new("ipc_latency_seconds", "IPC operation latency"),
            &["operation", "format"],
        ).expect("failed to create ipc_latency histogram");

        let file_ops = CounterVec::new(
            Opts::new("file_operations_total", "Total file operations"),
            &["operation", "status"],
        ).unwrap();

        let memory_usage = GaugeVec::new(
            Opts::new("memory_usage_bytes", "Memory usage in bytes"),
            &["component"],
        ).unwrap();

        let cpu_usage = GaugeVec::new(
            Opts::new("cpu_usage_percent", "CPU usage percentage"),
            &["component"],
        ).unwrap();

        let startup_time = Histogram::with_opts(
            HistogramOpts::new("startup_time_seconds", "Utility startup time"),
        ).expect("failed to create startup_time histogram");

        let git_cache_hits = Counter::new(
            "git_cache_hits_total", "Git cache hits",
        ).expect("failed to create git_cache_hits counter");

        let git_cache_misses = Counter::new(
            "git_cache_misses_total", "Git cache misses",
        ).expect("failed to create git_cache_misses counter");

        let active_connections = Gauge::new(
            "active_connections", "Number of active IPC connections",
        ).expect("failed to create active_connections gauge");

        // Register all metrics
        registry.register(Box::new(ipc_latency.clone())).unwrap();
        registry.register(Box::new(file_ops.clone())).unwrap();
        registry.register(Box::new(memory_usage.clone())).unwrap();
        registry.register(Box::new(cpu_usage.clone())).unwrap();
        registry.register(Box::new(startup_time.clone())).unwrap();
        registry.register(Box::new(git_cache_hits.clone())).unwrap();
        registry.register(Box::new(git_cache_misses.clone())).unwrap();
        registry.register(Box::new(active_connections.clone())).unwrap();

        Self {
            registry: Arc::new(registry),
            ipc_latency: Arc::new(ipc_latency),
            file_ops: Arc::new(file_ops),
            memory_usage: Arc::new(memory_usage),
            cpu_usage: Arc::new(cpu_usage),
            startup_time: Arc::new(startup_time),
            git_cache_hits: Arc::new(git_cache_hits),
            git_cache_misses: Arc::new(git_cache_misses),
            active_connections: Arc::new(active_connections),
        }
    }

    pub fn record_ipc_latency(&self, operation: &str, format: &str, duration: Duration) {
        self.ipc_latency
            .with_label_values(&[operation, format])
            .observe(duration.as_secs_f64());
    }

    pub fn record_file_operation(&self, operation: &str, success: bool) {
        let status = if success { "success" } else { "failure" };
        self.file_ops
            .with_label_values(&[operation, status])
            .inc();
    }

    pub fn update_memory_usage(&self, component: &str, bytes: f64) {
        self.memory_usage
            .with_label_values(&[component])
            .set(bytes);
    }

    pub fn update_cpu_usage(&self, component: &str, percent: f64) {
        self.cpu_usage
            .with_label_values(&[component])
            .set(percent);
    }

    pub fn record_startup_time(&self, duration: Duration) {
        self.startup_time.observe(duration.as_secs_f64());
    }

    pub fn record_git_cache_hit(&self) {
        self.git_cache_hits.inc();
    }

    pub fn record_git_cache_miss(&self) {
        self.git_cache_misses.inc();
    }

    pub fn set_active_connections(&self, count: f64) {
        self.active_connections.set(count);
    }

    pub fn export_metrics(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

/// Real-time performance monitor
pub struct PerfMonitor {
    metrics: Arc<RwLock<PerformanceMetrics>>,
    collector: Arc<MetricsCollector>,
    system: System,
    shutdown: mpsc::Sender<()>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: SystemTime,
    pub ipc_latency_p50: f64,
    pub ipc_latency_p99: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub file_ops_per_sec: f64,
    pub git_cache_hit_rate: f64,
    pub active_connections: usize,
    pub startup_time_avg: f64,
}

impl PerfMonitor {
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

        let monitor = Self {
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            collector,
            system: System::new(),
            shutdown: shutdown_tx,
        };

        // Start monitoring loop
        let metrics = monitor.metrics.clone();
        let system = System::new();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::update_metrics(&metrics, &system);
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                }
            }
        });

        monitor
    }

    fn update_metrics(metrics: &Arc<RwLock<PerformanceMetrics>>, system: &System) {
        let mut m = metrics.write();

        m.timestamp = SystemTime::now();

        // Update system metrics
        if let Ok(memory) = system.memory() {
            let used = memory.total.as_u64() - memory.free.as_u64();
            m.memory_usage_mb = used as f64 / 1_048_576.0;
        }

        if let Ok(cpu) = system.cpu_load_aggregate() {
            std::thread::sleep(Duration::from_millis(100));
            if let Ok(cpu) = cpu.done() {
                m.cpu_usage_percent = f64::from(cpu.user + cpu.system) * 100.0;
            }
        }
    }

    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().clone()
    }

    pub async fn shutdown(self) {
        let _ = self.shutdown.send(()).await;
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            ipc_latency_p50: 0.0,
            ipc_latency_p99: 0.0,
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            file_ops_per_sec: 0.0,
            git_cache_hit_rate: 0.0,
            active_connections: 0,
            startup_time_avg: 0.0,
        }
    }
}

/// Performance alert system
pub struct AlertSystem {
    thresholds: Arc<RwLock<AlertThresholds>>,
    alerts: Arc<DashMap<String, Alert>>,
    subscribers: Arc<RwLock<Vec<mpsc::Sender<Alert>>>>,
}

#[derive(Clone, Debug)]
pub struct AlertThresholds {
    pub ipc_latency_ms: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub startup_time_ms: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            ipc_latency_ms: 50.0,
            memory_usage_mb: 100.0,
            cpu_usage_percent: 80.0,
            startup_time_ms: 500.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub component: String,
    pub message: String,
    pub timestamp: SystemTime,
    pub value: f64,
    pub threshold: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl AlertSystem {
    pub fn new() -> Self {
        Self {
            thresholds: Arc::new(RwLock::new(AlertThresholds::default())),
            alerts: Arc::new(DashMap::new()),
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn check_metrics(&self, metrics: &PerformanceMetrics) {
        let thresholds = self.thresholds.read();

        // Check IPC latency
        if metrics.ipc_latency_p99 > thresholds.ipc_latency_ms {
            self.create_alert(
                "ipc_latency",
                AlertSeverity::Warning,
                "IPC",
                format!("IPC latency P99 {}ms exceeds threshold {}ms",
                    metrics.ipc_latency_p99, thresholds.ipc_latency_ms),
                metrics.ipc_latency_p99,
                thresholds.ipc_latency_ms,
            );
        }

        // Check memory usage
        if metrics.memory_usage_mb > thresholds.memory_usage_mb {
            self.create_alert(
                "memory_usage",
                AlertSeverity::Warning,
                "Memory",
                format!("Memory usage {}MB exceeds threshold {}MB",
                    metrics.memory_usage_mb, thresholds.memory_usage_mb),
                metrics.memory_usage_mb,
                thresholds.memory_usage_mb,
            );
        }

        // Check CPU usage
        if metrics.cpu_usage_percent > thresholds.cpu_usage_percent {
            self.create_alert(
                "cpu_usage",
                AlertSeverity::Critical,
                "CPU",
                format!("CPU usage {}% exceeds threshold {}%",
                    metrics.cpu_usage_percent, thresholds.cpu_usage_percent),
                metrics.cpu_usage_percent,
                thresholds.cpu_usage_percent,
            );
        }
    }

    fn create_alert(
        &self,
        id: &str,
        severity: AlertSeverity,
        component: &str,
        message: String,
        value: f64,
        threshold: f64,
    ) {
        let alert = Alert {
            id: id.to_string(),
            severity,
            component: component.to_string(),
            message,
            timestamp: SystemTime::now(),
            value,
            threshold,
        };

        self.alerts.insert(id.to_string(), alert.clone());
        self.notify_subscribers(alert);
    }

    fn notify_subscribers(&self, alert: Alert) {
        let subscribers = self.subscribers.read();
        for subscriber in subscribers.iter() {
            let _ = subscriber.try_send(alert.clone());
        }
    }

    pub async fn subscribe(&self) -> mpsc::Receiver<Alert> {
        let (tx, rx) = mpsc::channel(100);
        self.subscribers.write().push(tx);
        rx
    }

    pub fn get_active_alerts(&self) -> Vec<Alert> {
        self.alerts.iter().map(|e| e.value().clone()).collect()
    }

    pub fn clear_alert(&self, id: &str) {
        self.alerts.remove(id);
    }
}

/// Performance report generator
pub struct ReportGenerator {
    metrics_history: Arc<RwLock<Vec<PerformanceMetrics>>>,
    max_history: usize,
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            max_history: 1000,
        }
    }

    pub fn record(&self, metrics: PerformanceMetrics) {
        let mut history = self.metrics_history.write();
        history.push(metrics);

        if history.len() > self.max_history {
            history.remove(0);
        }
    }

    pub fn generate_report(&self) -> PerformanceReport {
        let history = self.metrics_history.read();

        if history.is_empty() {
            return PerformanceReport::default();
        }

        let mut report = PerformanceReport {
            generated_at: SystemTime::now(),
            duration: Duration::from_secs(history.len() as u64),
            ..Default::default()
        };

        // Calculate averages and percentiles
        let mut ipc_latencies: Vec<f64> = history.iter().map(|m| m.ipc_latency_p50).collect();
        let mut memory_usages: Vec<f64> = history.iter().map(|m| m.memory_usage_mb).collect();
        let mut cpu_usages: Vec<f64> = history.iter().map(|m| m.cpu_usage_percent).collect();

        ipc_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
        memory_usages.sort_by(|a, b| a.partial_cmp(b).unwrap());
        cpu_usages.sort_by(|a, b| a.partial_cmp(b).unwrap());

        report.ipc_latency_avg = average(&ipc_latencies);
        report.ipc_latency_p50 = percentile(&ipc_latencies, 0.5);
        report.ipc_latency_p99 = percentile(&ipc_latencies, 0.99);

        report.memory_usage_avg = average(&memory_usages);
        report.memory_usage_max = memory_usages.last().copied().unwrap_or(0.0);

        report.cpu_usage_avg = average(&cpu_usages);
        report.cpu_usage_max = cpu_usages.last().copied().unwrap_or(0.0);

        report
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub generated_at: SystemTime,
    pub duration: Duration,
    pub ipc_latency_avg: f64,
    pub ipc_latency_p50: f64,
    pub ipc_latency_p99: f64,
    pub memory_usage_avg: f64,
    pub memory_usage_max: f64,
    pub cpu_usage_avg: f64,
    pub cpu_usage_max: f64,
    pub recommendations: Vec<String>,
}

impl Default for PerformanceReport {
    fn default() -> Self {
        Self {
            generated_at: SystemTime::now(),
            duration: Duration::default(),
            ipc_latency_avg: 0.0,
            ipc_latency_p50: 0.0,
            ipc_latency_p99: 0.0,
            memory_usage_avg: 0.0,
            memory_usage_max: 0.0,
            cpu_usage_avg: 0.0,
            cpu_usage_max: 0.0,
            recommendations: Vec::new(),
        }
    }
}

fn average(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn percentile(sorted_values: &[f64], p: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let index = ((sorted_values.len() - 1) as f64 * p) as usize;
    sorted_values[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        collector.record_ipc_latency("send", "json", Duration::from_millis(10));
        collector.record_file_operation("read", true);
        collector.update_memory_usage("daemon", 50_000_000.0);
        collector.record_startup_time(Duration::from_millis(250));

        let metrics = collector.export_metrics();
        assert!(metrics.contains("ipc_latency_seconds"));
        assert!(metrics.contains("file_operations_total"));
    }

    #[test]
    fn test_alert_system() {
        let alert_system = AlertSystem::new();

        let metrics = PerformanceMetrics {
            cpu_usage_percent: 90.0,
            memory_usage_mb: 150.0,
            ..Default::default()
        };

        alert_system.check_metrics(&metrics);

        let alerts = alert_system.get_active_alerts();
        assert!(!alerts.is_empty());
    }

    #[test]
    fn test_report_generator() {
        let generator = ReportGenerator::new();

        for i in 0..10 {
            let metrics = PerformanceMetrics {
                ipc_latency_p50: i as f64 * 10.0,
                memory_usage_mb: 50.0 + i as f64,
                cpu_usage_percent: 20.0 + i as f64 * 2.0,
                ..Default::default()
            };
            generator.record(metrics);
        }

        let report = generator.generate_report();
        assert!(report.ipc_latency_avg > 0.0);
        assert!(report.memory_usage_avg > 50.0);
    }
}