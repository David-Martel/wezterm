//! Directory walking with .gitignore support using the `ignore` crate.
//!
//! This module provides high-performance directory traversal that respects
//! .gitignore files and supports filtering by file type.

use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Options for directory walking.
#[derive(Debug, Clone)]
pub struct WalkOptions {
    /// Whether to respect .gitignore files.
    pub respect_gitignore: bool,
    /// Whether to include hidden files.
    pub include_hidden: bool,
    /// Maximum depth to traverse (None for unlimited).
    pub max_depth: Option<usize>,
    /// Whether to follow symlinks.
    pub follow_symlinks: bool,
    /// File type names to include (e.g., "rust", "python"). Empty means all.
    pub file_types: Vec<String>,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            respect_gitignore: true,
            include_hidden: false,
            max_depth: None,
            follow_symlinks: false,
            file_types: vec![],
        }
    }
}

/// A directory entry from walking.
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// The full path to the entry.
    pub path: PathBuf,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Whether this is a symlink.
    pub is_symlink: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Depth from the root path.
    pub depth: usize,
}

impl DirEntry {
    /// Get the file name as a string.
    pub fn file_name(&self) -> Option<&str> {
        self.path.file_name().and_then(|n| n.to_str())
    }

    /// Get the file extension as a string.
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }

    /// Check if this entry is a file (not a directory).
    pub fn is_file(&self) -> bool {
        !self.is_dir
    }
}

/// Directory walker with .gitignore support.
///
/// Uses the `ignore` crate (same as ripgrep) for efficient traversal.
pub struct Walker {
    options: WalkOptions,
}

impl Walker {
    /// Create a new walker with default options.
    pub fn new() -> Self {
        Self {
            options: WalkOptions::default(),
        }
    }

    /// Create a new walker with custom options.
    pub fn with_options(options: WalkOptions) -> Self {
        Self { options }
    }

    /// Walk a directory and collect all entries.
    pub fn walk<P: AsRef<Path>>(&self, root: P) -> Result<Vec<DirEntry>> {
        let mut entries = Vec::new();
        for entry in self.walk_iter(root) {
            entries.push(entry?);
        }
        Ok(entries)
    }

    /// Walk a directory and return an iterator.
    pub fn walk_iter<P: AsRef<Path>>(&self, root: P) -> impl Iterator<Item = Result<DirEntry>> {
        let mut builder = WalkBuilder::new(root.as_ref());

        builder
            .git_ignore(self.options.respect_gitignore)
            .git_global(self.options.respect_gitignore)
            .git_exclude(self.options.respect_gitignore)
            .hidden(!self.options.include_hidden)
            .follow_links(self.options.follow_symlinks);

        if let Some(depth) = self.options.max_depth {
            builder.max_depth(Some(depth));
        }

        // Add file type filters if specified
        if !self.options.file_types.is_empty() {
            let mut types_builder = ignore::types::TypesBuilder::new();
            types_builder.add_defaults();

            for ft in &self.options.file_types {
                // select() returns &mut TypesBuilder, not a Result
                types_builder.select(ft);
            }

            if let Ok(types) = types_builder.build() {
                builder.types(types);
            }
        }

        builder.build().filter_map(|result| match result {
            Ok(entry) => {
                let path = entry.path().to_path_buf();
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        log::debug!("Failed to get metadata for {:?}: {}", path, e);
                        return None;
                    }
                };

                Some(Ok(DirEntry {
                    path,
                    is_dir: metadata.is_dir(),
                    is_symlink: metadata.file_type().is_symlink(),
                    size: if metadata.is_file() {
                        metadata.len()
                    } else {
                        0
                    },
                    depth: entry.depth(),
                }))
            }
            Err(e) => {
                log::debug!("Walk error: {}", e);
                Some(Err(e.into()))
            }
        })
    }

    /// Walk only files (skip directories) and return an iterator.
    pub fn walk_files<P: AsRef<Path>>(&self, root: P) -> impl Iterator<Item = Result<DirEntry>> {
        self.walk_iter(root)
            .filter(|r| r.as_ref().map(|e| e.is_file()).unwrap_or(true))
    }

    /// Walk a directory and call a callback for each entry.
    ///
    /// Returns early if the callback returns `false`.
    pub fn walk_with_callback<P, F>(&self, root: P, mut callback: F) -> Result<()>
    where
        P: AsRef<Path>,
        F: FnMut(DirEntry) -> bool,
    {
        for entry in self.walk_iter(root) {
            let entry = entry?;
            if !callback(entry) {
                break;
            }
        }
        Ok(())
    }
}

impl Default for Walker {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Walker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Walker")
            .field("options", &self.options)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_basic_walk() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // Create test structure
        File::create(root.join("file1.txt")).unwrap();
        File::create(root.join("file2.rs")).unwrap();
        fs::create_dir(root.join("subdir")).unwrap();
        File::create(root.join("subdir/file3.txt")).unwrap();

        let walker = Walker::new();
        let entries = walker.walk(root).unwrap();

        // Should find root + 2 files + subdir + 1 file in subdir = 5 entries
        assert!(entries.len() >= 4);
    }

    #[test]
    fn test_max_depth() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("a/b/c")).unwrap();
        File::create(root.join("a/b/c/deep.txt")).unwrap();

        let options = WalkOptions {
            max_depth: Some(2),
            ..Default::default()
        };
        let walker = Walker::with_options(options);
        let entries = walker.walk(root).unwrap();

        // Should not include files deeper than depth 2
        assert!(entries.iter().all(|e| e.depth <= 2));
    }

    #[test]
    fn test_hidden_files() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        File::create(root.join("visible.txt")).unwrap();
        File::create(root.join(".hidden")).unwrap();

        // Default: exclude hidden
        let walker = Walker::new();
        let entries = walker.walk(root).unwrap();
        assert!(!entries.iter().any(|e| e.file_name() == Some(".hidden")));

        // Include hidden
        let options = WalkOptions {
            include_hidden: true,
            ..Default::default()
        };
        let walker = Walker::with_options(options);
        let entries = walker.walk(root).unwrap();
        assert!(entries.iter().any(|e| e.file_name() == Some(".hidden")));
    }

    #[test]
    fn test_walk_files_only() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        File::create(root.join("file1.txt")).unwrap();
        fs::create_dir(root.join("subdir")).unwrap();
        File::create(root.join("subdir/file2.txt")).unwrap();

        let walker = Walker::new();
        let files: Vec<_> = walker.walk_files(root).collect();

        // Should only include files, not directories
        assert!(files.iter().all(|r| r.as_ref().map(|e| !e.is_dir).unwrap_or(true)));
    }

    #[test]
    fn test_dir_entry_methods() {
        let entry = DirEntry {
            path: PathBuf::from("/test/file.rs"),
            is_dir: false,
            is_symlink: false,
            size: 1024,
            depth: 2,
        };

        assert_eq!(entry.file_name(), Some("file.rs"));
        assert_eq!(entry.extension(), Some("rs"));
        assert!(entry.is_file());
    }

    #[test]
    fn test_walk_with_callback() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        File::create(root.join("file1.txt")).unwrap();
        File::create(root.join("file2.txt")).unwrap();
        File::create(root.join("file3.txt")).unwrap();

        let walker = Walker::new();
        let mut count = 0;

        // Stop after 2 entries
        walker
            .walk_with_callback(root, |_entry| {
                count += 1;
                count < 2
            })
            .unwrap();

        assert_eq!(count, 2);
    }
}
