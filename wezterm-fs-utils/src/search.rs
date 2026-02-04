//! Fuzzy file search using the nucleo-matcher crate.
//!
//! This module provides fuzzy matching capabilities similar to
//! wezterm-fs-explorer/src/search.rs but with a simpler API using
//! nucleo-matcher directly (avoiding the full Nucleo async pipeline).

use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
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
    /// Indices of matched characters in the item.
    pub indices: Vec<u32>,
}

impl SearchMatch {
    /// Check if a character at the given index was matched.
    pub fn is_matched(&self, index: u32) -> bool {
        self.indices.contains(&index)
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
    pub fn match_items<I, S>(&mut self, pattern: &str, items: I) -> Vec<SearchMatch>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut matches: Vec<SearchMatch> = items
            .into_iter()
            .filter_map(|item| self.match_item(pattern, item.as_ref()))
            .collect();

        // Sort by score (descending)
        matches.sort_by(|a, b| b.score.cmp(&a.score));

        // Limit results
        matches.truncate(self.options.max_results);

        matches
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

        let items = vec!["main.rs", "main_test.rs", "something_main.rs", "maintainer.rs"];
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
}
