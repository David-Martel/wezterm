use nucleo::{Config, Nucleo};
use std::path::PathBuf;

/// Result from a fuzzy search operation
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The matching path
    pub path: PathBuf,
    /// Indices of matched characters in the file name for highlighting
    /// Reserved for future highlighting functionality - not yet implemented
    #[allow(dead_code)]
    pub indices: Vec<u32>,
}

/// Fuzzy file search using the nucleo matcher (used by Helix editor)
pub struct FuzzySearch {
    matcher: Nucleo<PathBuf>,
}

impl FuzzySearch {
    /// Create a new fuzzy search instance
    pub fn new() -> Self {
        let config = Config::DEFAULT;
        Self {
            matcher: Nucleo::new(config, std::sync::Arc::new(|| {}), None, 1),
        }
    }

    /// Add items to the search index
    pub fn populate(&mut self, items: Vec<PathBuf>) {
        let injector = self.matcher.injector();
        for item in items {
            injector.push(item, |path, cols| {
                // Index by file name (last component)
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    cols[0] = name.into();
                }
            });
        }
    }

    /// Search with a query string and return top N matches
    pub fn search(&mut self, query: &str, limit: usize) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        // Perform the search
        self.matcher.pattern.reparse(
            0,
            query,
            nucleo::pattern::CaseMatching::Ignore,
            nucleo::pattern::Normalization::Smart,
            false,
        );

        // Tick the matcher to process items - loop until items are processed
        // Nucleo processes items asynchronously, so we need to wait
        for _ in 0..100 {
            let status = self.matcher.tick(10);
            if status.changed || status.running {
                continue;
            }
            break;
        }

        // Get the snapshot of matches
        let snapshot = self.matcher.snapshot();

        let matched_count = snapshot.matched_item_count();
        let limit_u32 = limit.min(matched_count as usize) as u32;

        let mut results = Vec::new();
        for item in snapshot.matched_items(..limit_u32) {
            let path = item.data.clone();

            // Note: Match indices extraction from Utf32String is complex
            // For now, we return empty indices and can enhance later with highlighting
            let indices = Vec::new();

            results.push(SearchResult { path, indices });
        }

        results
    }

    /// Clear the search index
    pub fn clear(&mut self) {
        // Create a new matcher to clear all items
        let config = Config::DEFAULT;
        self.matcher = Nucleo::new(config, std::sync::Arc::new(|| {}), None, 1);
    }
}

impl Default for FuzzySearch {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for FuzzySearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuzzySearch").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_search_basic() {
        let mut search = FuzzySearch::new();

        let items = vec![
            PathBuf::from("/home/user/documents/readme.txt"),
            PathBuf::from("/home/user/documents/README.md"),
            PathBuf::from("/home/user/pictures/photo.jpg"),
            PathBuf::from("/home/user/code/main.rs"),
        ];

        search.populate(items);

        let results = search.search("readme", 10);
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| {
            r.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_lowercase().contains("readme"))
                .unwrap_or(false)
        }));
    }

    #[test]
    fn test_fuzzy_search_empty_query() {
        let mut search = FuzzySearch::new();

        let items = vec![PathBuf::from("/test/file.txt")];

        search.populate(items);

        let results = search.search("", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_fuzzy_search_clear() {
        let mut search = FuzzySearch::new();

        let items = vec![PathBuf::from("/test/file.txt")];

        search.populate(items);
        search.clear();

        let results = search.search("file", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_result_debug() {
        let result = SearchResult {
            path: PathBuf::from("/test/file.txt"),
            indices: vec![0, 1, 2, 3],
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("SearchResult"));
    }

    #[test]
    fn test_fuzzy_search_debug() {
        let search = FuzzySearch::new();
        let debug_str = format!("{:?}", search);
        assert!(debug_str.contains("FuzzySearch"));
    }
}
