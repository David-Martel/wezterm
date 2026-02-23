//! Performance optimization library for WezTerm utilities
//!
//! This library provides high-performance implementations for:
//! - IPC communication with connection pooling and message batching
//! - File system operations with caching and parallel processing
//! - Git integration with incremental updates
//! - Memory management with object pooling
//! - Startup optimization with lazy loading

#![allow(dead_code)]

pub mod ipc;
pub mod fs;
pub mod git;
pub mod memory;
pub mod startup;
pub mod monitoring;

// Re-export commonly used types
pub use ipc::{IpcClient, ConnectionPool, MessageBatcher};
pub use fs::{DirectoryScanner, FileCache, DebouncedWatcher};
pub use git::{GitStatusCache, GitOperations};
pub use memory::{MemoryPool, BufferPool, ObjectPool};
pub use startup::{LazyInitializer, StartupOptimizer};
pub use monitoring::{PerfMonitor, MetricsCollector};