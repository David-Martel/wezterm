//! Performance optimization library for WezTerm utilities
//!
//! This library provides high-performance implementations for:
//! - IPC communication with connection pooling and message batching
//! - File system operations with caching and parallel processing
//! - Git integration with incremental updates
//! - Memory management with object pooling
//! - Startup optimization with lazy loading

#![allow(dead_code)]

pub mod fs;
pub mod git;
pub mod ipc;
pub mod memory;
pub mod monitoring;
pub mod startup;

// Re-export commonly used types
pub use fs::{DebouncedWatcher, DirectoryScanner, FileCache};
pub use git::{GitOperations, GitStatusCache};
pub use ipc::{ConnectionPool, IpcClient, MessageBatcher};
pub use memory::{BufferPool, MemoryPool, ObjectPool};
pub use monitoring::{MetricsCollector, PerfMonitor};
pub use startup::{LazyInitializer, StartupOptimizer};
