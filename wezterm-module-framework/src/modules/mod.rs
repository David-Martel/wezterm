//! Built-in modules for WezTerm.
//!
//! This module provides the built-in modules that ship with WezTerm:
//! - `fs_explorer`: Interactive filesystem explorer pane
//! - `watcher`: Background file watching service

pub mod fs_explorer;
pub mod watcher;

pub use fs_explorer::FsExplorerModule;
pub use watcher::WatcherModule;
