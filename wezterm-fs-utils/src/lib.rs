//! # WezTerm Filesystem Utilities
//!
//! High-performance filesystem operations for WezTerm modules.
//!
//! ## Features
//!
//! - **Walker**: Gitignore-aware directory traversal using the [`ignore`] crate
//! - **Search**: Fuzzy file matching using [`nucleo-matcher`] (Helix editor's engine)
//! - **Watcher**: Debounced file watching with [`notify`]
//!
//! ## Thread Safety
//!
//! All types in this crate are `Send + Sync` and safe to share across threads.
//!
//! ## Performance
//!
//! - **Search**: Uses bounded heap for O(n log k) top-K results
//! - **Walker**: Parallel directory traversal respecting `.gitignore`
//! - **Watcher**: Debounced events to reduce noise
//!
//! ## Example
//!
//! ```rust,no_run
//! use wezterm_fs_utils::{Walker, WalkOptions, FuzzyMatcher};
//! use std::path::PathBuf;
//!
//! // Walk directory respecting .gitignore
//! let walker = Walker::new();
//! let entries = walker.walk(PathBuf::from("/path/to/project")).unwrap();
//!
//! // Collect file paths
//! let paths: Vec<String> = entries
//!     .iter()
//!     .filter_map(|e| e.path.to_str().map(String::from))
//!     .collect();
//!
//! // Fuzzy search results
//! let mut matcher = FuzzyMatcher::new();
//! let matches = matcher.match_items("main", paths);
//!
//! for m in matches {
//!     println!("{} (score: {})", m.item, m.score);
//! }
//! ```
//!
//! ## Module Overview
//!
//! | Module | Description | Key Types |
//! |--------|-------------|-----------|
//! | [`search`] | Fuzzy file matching | [`FuzzyMatcher`], [`SearchMatch`] |
//! | [`walker`] | Directory traversal | [`Walker`], [`DirEntry`] |
//! | [`watcher`] | File system watching | [`Watcher`], [`WatchEvent`] |
//!
//! [`ignore`]: https://docs.rs/ignore
//! [`nucleo-matcher`]: https://docs.rs/nucleo-matcher
//! [`notify`]: https://docs.rs/notify

pub mod search;
pub mod walker;
pub mod watcher;

// Re-export main types for convenience
pub use search::{FuzzyMatcher, SearchMatch, SearchOptions};
pub use walker::{DirEntry, WalkOptions, Walker};
pub use watcher::{WatchEvent, WatchEventKind, WatchHandle, Watcher};
