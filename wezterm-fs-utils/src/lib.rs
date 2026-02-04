//! Consolidated filesystem utilities for WezTerm modules.
//!
//! This crate provides reusable filesystem operations including:
//! - Directory walking with .gitignore support (using `ignore` crate)
//! - Fuzzy file search using nucleo-matcher (same engine as Helix editor)
//! - File watching with debouncing and gitignore filtering
//!
//! These utilities are designed to be shared between wezterm-module-framework
//! components like the fs-explorer and file watcher modules.

pub mod search;
pub mod walker;
pub mod watcher;

// Re-export main types for convenience
pub use search::{FuzzyMatcher, SearchMatch, SearchOptions};
pub use walker::{DirEntry, WalkOptions, Walker};
pub use watcher::{WatchEvent, WatchEventKind, WatchHandle, Watcher};
