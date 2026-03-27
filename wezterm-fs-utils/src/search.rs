//! Fuzzy file search using the nucleo-matcher crate.
//!
//! This module provides fuzzy matching capabilities similar to
//! wezterm-fs-explorer/src/search.rs but with a simpler API using
//! nucleo-matcher directly (avoiding the full Nucleo async pipeline).
//!
//! ## Performance Optimizations
//!
//! - **Bounded heap**: Uses min-heap to keep only top N results, achieving
//!   O(n log k) complexity instead of O(n log n) for k results.
//! - **Binary search**: Match indices are sorted, enabling O(log n) lookups.
//! - **Buffer reuse**: UTF-32 conversion buffer is reused across matches.

use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::path::Path;

/// Options for fuzzy search.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Whether the search is case-sensitive.
    pub case_sensitive: bool,
    /// Maximum number of results to return.
    pub max_results: usize,
    /// Minimum score threshold (scores range roughly 0-1000).
    pub min_score: u32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            max_results: 100,
            min_score: 0,
        }
    }
}

/// A search match result.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// The matched item (path string or other text).
    pub item: String,
    /// The match score (higher is better).
    pub score: u32,
    /// Indices of matched characters in the item (sorted).
    pub indices: Vec<u32>,
}

impl SearchMatch {
    /// Check if a character at the given index was matched.
    ///
    /// Uses binary search for O(log n) performance since indices are sorted.
    #[inline]
    pub fn is_matched(&self, index: u32) -> bool {
        self.indices.binary_search(&index).is_ok()
    }
}

// Implement ordering traits for heap operations.
// We order by score for use in a max-heap (or Reverse for min-heap).
impl PartialEq for SearchMatch {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}

impl Eq for SearchMatch {}

impl PartialOrd for SearchMatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchMatch {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

/// Fuzzy matcher for file paths and text.
///
/// Uses nucleo-matcher (same engine as Helix editor) for high-quality
/// fuzzy matching optimized for file paths.
pub struct FuzzyMatcher {
    matcher: Matcher,
    options: SearchOptions,
    // Reusable buffer for UTF-32 conversion (performance optimization)
    buf: Vec<char>,
}

impl FuzzyMatcher {
    /// Create a new fuzzy matcher with default options.
    pub fn new() -> Self {
        Self::with_options(SearchOptions::default())
    }

    /// Create a new fuzzy matcher with custom options.
    pub fn with_options(options: SearchOptions) -> Self {
        // Use path-matching config for better file path scoring
        let config = Config::DEFAULT.match_paths();
        Self {
            matcher: Matcher::new(config),
            options,
            buf: Vec::with_capacity(256),
        }
    }

    /// Match a single item against a pattern.
    pub fn match_item(&mut self, pattern: &str, item: &str) -> Option<SearchMatch> {
        if pattern.is_empty() {
            return Some(SearchMatch {
                item: item.to_string(),
                score: 0,
                indices: vec![],
            });
        }

        let case_matching = if self.options.case_sensitive {
            CaseMatching::Respect
        } else {
            CaseMatching::Smart
        };

        // Create pattern using the simpler parse API
        let pat = Pattern::new(
            pattern,
            case_matching,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );

        // Reuse buffer for UTF-32 conversion
        self.buf.clear();
        let haystack = Utf32Str::new(item, &mut self.buf);

        let score = pat.score(haystack, &mut self.matcher)?;

        if score < self.options.min_score {
            return None;
        }

        // Get match indices
        let mut indices = Vec::new();
        pat.indices(haystack, &mut self.matcher, &mut indices);
        indices.sort_unstable();
        indices.dedup();

        Some(SearchMatch {
            item: item.to_string(),
            score,
            indices,
        })
    }

    /// Match multiple items against a pattern and return sorted results.
    ///
    /// Uses a bounded min-heap to keep only the top `max_results` matches,
    /// achieving O(n log k) complexity instead of O(n log n) where k is
    /// the maximum number of results.
    pub fn match_items<I, S>(&mut self, pattern: &str, items: I) -> Vec<SearchMatch>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let max = self.options.max_results;

        // Special case: if max is 0, return empty
        if max == 0 {
            return Vec::new();
        }

        // Use a min-heap (via Reverse) to keep only top N results.
        // The heap stores Reverse<SearchMatch> so the minimum score is at top.
        // When heap exceeds capacity, we pop the minimum (lowest score).
        let mut heap: BinaryHeap<Reverse<SearchMatch>> = BinaryHeap::with_capacity(max + 1);

        for item in items {
            if let Some(m) = self.match_item(pattern, item.as_ref()) {
                heap.push(Reverse(m));

                // If heap exceeds capacity, remove the lowest score
                if heap.len() > max {
                    heap.pop();
                }
            }
        }

        // Extract results in descending score order
        let mut results: Vec<SearchMatch> = heap.into_iter().map(|Reverse(m)| m).collect();
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    /// Match file paths against a pattern.
    ///
    /// This is a convenience method that extracts file names for matching
    /// but returns full paths in the results.
    pub fn match_paths<'a, I>(&mut self, pattern: &str, paths: I) -> Vec<SearchMatch>
    where
        I: IntoIterator<Item = &'a Path>,
    {
        let path_strs: Vec<_> = paths
            .into_iter()
            .filter_map(|p| p.to_str().map(|s| s.to_string()))
            .collect();

        self.match_items(pattern, path_strs)
    }

    /// Update the options for this matcher.
    pub fn set_options(&mut self, options: SearchOptions) {
        self.options = options;
    }
}

impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for FuzzyMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FuzzyMatcher")
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_basic_fuzzy_match() {
        let mut matcher = FuzzyMatcher::new();

        let result = matcher.match_item("main", "src/main.rs");
        assert!(result.is_some());

        let m = result.unwrap();
        assert!(m.score > 0);
        assert!(!m.indices.is_empty());
    }

    #[test]
    fn test_no_match() {
        let mut matcher = FuzzyMatcher::new();

        // This should not match because there's no 'x', 'y', 'z' sequence
        let result = matcher.match_item("xyz", "abc");
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_pattern_matches_all() {
        let mut matcher = FuzzyMatcher::new();

        let result = matcher.match_item("", "anything");
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 0);
    }

    #[test]
    fn test_match_multiple() {
        let mut matcher = FuzzyMatcher::new();

        let items = vec!["src/main.rs", "src/lib.rs", "Cargo.toml", "README.md"];
        let matches = matcher.match_items("rs", items);

        // Should match .rs files with higher scores
        assert!(matches.len() >= 2);
    }

    #[test]
    fn test_match_paths() {
        let mut matcher = FuzzyMatcher::new();

        let paths: Vec<PathBuf> = vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/lib.rs"),
            PathBuf::from("tests/test.rs"),
        ];
        let path_refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();

        let matches = matcher.match_paths("test", path_refs);

        assert!(!matches.is_empty());
        assert!(matches[0].item.contains("test"));
    }

    #[test]
    fn test_score_ordering() {
        let mut matcher = FuzzyMatcher::new();

        let items = vec![
            "main.rs",
            "main_test.rs",
            "something_main.rs",
            "maintainer.rs",
        ];
        let matches = matcher.match_items("main", items);

        // First result should be exact prefix match "main.rs"
        assert!(matches.len() >= 2);
        assert!(matches[0].score >= matches[1].score);
    }

    #[test]
    fn test_case_insensitive_default() {
        let mut matcher = FuzzyMatcher::new();

        let result = matcher.match_item("main", "MAIN.RS");
        assert!(result.is_some());
    }

    #[test]
    fn test_min_score_threshold() {
        let options = SearchOptions {
            min_score: 1000, // Very high threshold
            ..Default::default()
        };
        let mut matcher = FuzzyMatcher::with_options(options);

        // This match should be filtered out due to low score
        let result = matcher.match_item("m", "abcdefghijklmnop");
        assert!(result.is_none());
    }

    #[test]
    fn test_debug_impl() {
        let matcher = FuzzyMatcher::new();
        let debug_str = format!("{:?}", matcher);
        assert!(debug_str.contains("FuzzyMatcher"));
    }

    #[test]
    fn test_is_matched_binary_search() {
        let m = SearchMatch {
            item: "test".to_string(),
            score: 100,
            indices: vec![0, 2, 5, 10, 15, 20],
        };

        // Test found indices
        assert!(m.is_matched(0));
        assert!(m.is_matched(5));
        assert!(m.is_matched(20));

        // Test not found indices
        assert!(!m.is_matched(1));
        assert!(!m.is_matched(3));
        assert!(!m.is_matched(100));
    }

    #[test]
    fn test_bounded_heap_limits_results() {
        let options = SearchOptions {
            max_results: 3,
            ..Default::default()
        };
        let mut matcher = FuzzyMatcher::with_options(options);

        // Create many items that will all match
        let items: Vec<String> = (0..100).map(|i| format!("file{}.rs", i)).collect();

        let matches = matcher.match_items("file", items);

        // Should only return max_results items
        assert_eq!(matches.len(), 3);

        // Results should be sorted by score (descending)
        for i in 0..matches.len() - 1 {
            assert!(matches[i].score >= matches[i + 1].score);
        }
    }

    #[test]
    fn test_empty_max_results() {
        let options = SearchOptions {
            max_results: 0,
            ..Default::default()
        };
        let mut matcher = FuzzyMatcher::with_options(options);

        let items = vec!["a.rs", "b.rs", "c.rs"];
        let matches = matcher.match_items("rs", items);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_search_match_ordering() {
        let m1 = SearchMatch {
            item: "low".to_string(),
            score: 10,
            indices: vec![],
        };
        let m2 = SearchMatch {
            item: "high".to_string(),
            score: 100,
            indices: vec![],
        };
        let m3 = SearchMatch {
            item: "equal".to_string(),
            score: 100,
            indices: vec![],
        };

        assert!(m2 > m1);
        assert!(m1 < m2);
        assert_eq!(m2.cmp(&m3), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_heap_keeps_top_scores() {
        let options = SearchOptions {
            max_results: 2,
            ..Default::default()
        };
        let mut matcher = FuzzyMatcher::with_options(options);

        // Items with different expected match qualities
        // "main.rs" should score highest for pattern "main"
        let items = vec![
            "something.txt",
            "main.rs",
            "contains_main_word.rs",
            "nomatch.txt",
            "main_test.rs",
        ];

        let matches = matcher.match_items("main", items);

        assert_eq!(matches.len(), 2);
        // Top results should contain "main"
        assert!(matches.iter().all(|m| m.item.contains("main")));
    }
}
