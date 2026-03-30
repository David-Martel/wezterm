//! Path translation utilities for seamless Windows/WSL path conversions.
//!
//! This module provides utilities to convert between Windows and WSL paths,
//! detect path types, and normalize paths to the current platform format.
//!
//! # Examples
//!
//! ```
//! use std::path::Path;
//! use wezterm_fs_explorer::path_utils::{to_wsl_path, to_windows_path, detect_path_type, PathType};
//!
//! // Convert Windows path to WSL
//! let win_path = Path::new(r"C:\Users\david\file.txt");
//! let wsl_path = to_wsl_path(win_path);
//! assert_eq!(wsl_path.to_str().unwrap(), "/mnt/c/Users/david/file.txt");
//!
//! // Convert WSL path to Windows
//! let wsl_path = Path::new("/mnt/c/Users/david/file.txt");
//! let win_path = to_windows_path(wsl_path);
//! assert_eq!(win_path.to_str().unwrap(), r"C:\Users\david\file.txt");
//!
//! // Detect path type
//! assert_eq!(detect_path_type(Path::new(r"C:\path")), PathType::Windows);
//! assert_eq!(detect_path_type(Path::new("/mnt/c/path")), PathType::Wsl);
//! ```

// Library module - items are exported for external consumers
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Represents the type of a filesystem path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathType {
    /// Windows-style path (e.g., C:\Users\name or \\server\share)
    Windows,
    /// WSL-style path (e.g., /mnt/c/Users/name)
    Wsl,
    /// Unix-style path (e.g., /home/user or relative paths)
    Unix,
    /// Path type cannot be determined
    Unknown,
}

/// Detects the type of a filesystem path.
///
/// This function analyzes the path structure to determine if it's a Windows path,
/// WSL path, Unix path, or unknown format.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use wezterm_fs_explorer::path_utils::{detect_path_type, PathType};
///
/// assert_eq!(detect_path_type(Path::new(r"C:\Windows")), PathType::Windows);
/// assert_eq!(detect_path_type(Path::new("/mnt/c/Windows")), PathType::Wsl);
/// assert_eq!(detect_path_type(Path::new("/home/user")), PathType::Unix);
/// assert_eq!(detect_path_type(Path::new("relative/path")), PathType::Unix);
/// ```
pub fn detect_path_type(path: &Path) -> PathType {
    let path_str = path.to_string_lossy();

    // Check for UNC paths (\\server\share)
    if path_str.starts_with(r"\\") || path_str.starts_with("//") {
        return PathType::Windows;
    }

    // Check for Windows absolute paths with drive letters (C:, C:\, D:\, etc.)
    // PERFORMANCE: Use iterator to avoid allocating Vec<char> for entire path
    if path_str.len() >= 2 {
        let mut chars = path_str.chars();
        if let (Some(c0), Some(c1)) = (chars.next(), chars.next()) {
            if c0.is_ascii_alphabetic() && c1 == ':' {
                // Could be C:, C:\, C:/, etc.
                let c2 = chars.next();
                if c2.is_none() || c2 == Some('\\') || c2 == Some('/') {
                    return PathType::Windows;
                }
            }
        }
    }

    // Check for WSL paths (/mnt/[a-z] or /mnt/[a-z]/)
    if path_str.starts_with("/mnt/") && path_str.len() >= 6 {
        let drive_char = path_str.chars().nth(5);
        if let Some(c) = drive_char {
            if c.is_ascii_lowercase() {
                // Accept /mnt/c, /mnt/c/, or /mnt/c/path
                let next_char = path_str.chars().nth(6);
                if next_char == Some('/') || next_char.is_none() {
                    return PathType::Wsl;
                }
            }
        }
    }

    // Check for Unix absolute paths
    if path_str.starts_with('/') {
        return PathType::Unix;
    }

    // Relative paths are treated as Unix-style
    PathType::Unix
}

/// Converts a Windows path to WSL format.
///
/// Converts drive letters (C:\) to WSL mount points (/mnt/c/) and handles
/// backslashes, UNC paths, and special characters.
///
/// # Behavior
///
/// - Windows paths (C:\path) → /mnt/c/path
/// - UNC paths (\\server\share) → //server/share
/// - Backslashes → Forward slashes
/// - Already-converted WSL paths are returned as-is (idempotent)
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use wezterm_fs_explorer::path_utils::to_wsl_path;
///
/// let path = Path::new(r"C:\Users\david\file.txt");
/// assert_eq!(to_wsl_path(path).to_str().unwrap(), "/mnt/c/Users/david/file.txt");
///
/// // Handles spaces and special characters
/// let path = Path::new(r"C:\Program Files\My App\file.txt");
/// let wsl = to_wsl_path(path);
/// assert_eq!(wsl.to_str().unwrap(), "/mnt/c/Program Files/My App/file.txt");
/// ```
pub fn to_wsl_path(path: &Path) -> PathBuf {
    let path_type = detect_path_type(path);

    // Already a WSL or Unix path - return as-is
    if path_type == PathType::Wsl || path_type == PathType::Unix {
        return path.to_path_buf();
    }

    let path_str = path.to_string_lossy();

    // Handle UNC paths: \\server\share → //server/share
    if path_str.starts_with(r"\\") {
        let converted = path_str.replace('\\', "/");
        return PathBuf::from(converted);
    }

    // Handle Windows drive paths: C:, C:\, or C:\path → /mnt/c, /mnt/c/, or /mnt/c/path
    // PERFORMANCE: Use iterator to avoid allocating Vec<char> for entire path
    if path_str.len() >= 2 {
        let mut chars = path_str.chars();
        if let (Some(c0), Some(c1)) = (chars.next(), chars.next()) {
            if c0.is_ascii_alphabetic() && c1 == ':' {
                let drive = c0.to_ascii_lowercase();
                let rest = if path_str.len() > 2 {
                    &path_str[2..]
                } else {
                    ""
                };
                let rest_normalized = rest.replace('\\', "/");

                // Ensure path starts with / if there's content after the drive
                let rest_with_slash = if rest_normalized.is_empty() {
                    String::new()
                } else if rest_normalized.starts_with('/') {
                    rest_normalized
                } else {
                    format!("/{}", rest_normalized)
                };

                return PathBuf::from(format!("/mnt/{}{}", drive, rest_with_slash));
            }
        }
    }

    // Fallback: just normalize backslashes to forward slashes
    PathBuf::from(path_str.replace('\\', "/"))
}

/// Converts a WSL path to Windows format.
///
/// Converts WSL mount points (/mnt/c/) to Windows drive letters (C:\) and
/// handles forward slashes, Unix paths that cannot be converted, and special characters.
///
/// # Behavior
///
/// - WSL paths (/mnt/c/path) → C:\path
/// - Unix paths (/home/user) → \\wsl.localhost\Ubuntu\home\user
/// - Forward slashes → Backslashes
/// - Already-converted Windows paths are returned as-is (idempotent)
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use wezterm_fs_explorer::path_utils::to_windows_path;
///
/// let path = Path::new("/mnt/c/Users/david/file.txt");
/// assert_eq!(to_windows_path(path).to_str().unwrap(), r"C:\Users\david\file.txt");
///
/// // Unix paths get converted to WSL UNC format
/// let path = Path::new("/home/david/file.txt");
/// let win = to_windows_path(path);
/// assert!(win.to_str().unwrap().starts_with(r"\\wsl.localhost\"));
/// ```
pub fn to_windows_path(path: &Path) -> PathBuf {
    let path_type = detect_path_type(path);

    // Already a Windows path - return as-is
    if path_type == PathType::Windows {
        return path.to_path_buf();
    }

    let path_str = path.to_string_lossy();

    // Handle WSL paths: /mnt/c, /mnt/c/, /mnt/c/path → C:\, C:\, C:\path
    if path_type == PathType::Wsl {
        if let Some(rest) = path_str.strip_prefix("/mnt/") {
            if let Some(drive_char) = rest.chars().next() {
                if drive_char.is_ascii_lowercase() {
                    let remainder = if rest.len() > 1 { &rest[1..] } else { "" };

                    // Skip the leading slash if present
                    let remainder = remainder.strip_prefix('/').unwrap_or(remainder);

                    let drive_upper = drive_char.to_ascii_uppercase();

                    // Only add content if remainder is not empty
                    if remainder.is_empty() {
                        return PathBuf::from(format!(r"{}:\", drive_upper));
                    } else {
                        let rest_normalized = remainder.replace('/', r"\");
                        return PathBuf::from(format!(r"{}:\{}", drive_upper, rest_normalized));
                    }
                }
            }
        }
    }

    // Handle Unix paths: convert to WSL UNC path (\\wsl.localhost\Ubuntu\path)
    if path_type == PathType::Unix && path_str.starts_with('/') {
        let rest = &path_str[1..]; // Remove leading /
        let rest_normalized = rest.replace('/', r"\");
        return PathBuf::from(format!(r"\\wsl.localhost\Ubuntu\{}", rest_normalized));
    }

    // Fallback: just normalize forward slashes to backslashes
    PathBuf::from(path_str.replace('/', r"\"))
}

/// Normalizes a path to the current platform's native format.
///
/// On Windows, converts paths to Windows format. On Unix/WSL, converts to Unix format.
/// This is useful for ensuring paths work correctly with the current platform's APIs.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use wezterm_fs_explorer::path_utils::normalize_path;
///
/// // On Windows: converts to Windows format
/// let path = Path::new("/mnt/c/Users/david/file.txt");
/// let normalized = normalize_path(path);
///
/// #[cfg(windows)]
/// assert_eq!(normalized.to_str().unwrap(), r"C:\Users\david\file.txt");
///
/// #[cfg(not(windows))]
/// assert_eq!(normalized.to_str().unwrap(), "/mnt/c/Users/david/file.txt");
/// ```
pub fn normalize_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        to_windows_path(path)
    }

    #[cfg(not(windows))]
    {
        to_wsl_path(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_windows_paths() {
        assert_eq!(
            detect_path_type(Path::new(r"C:\Windows\System32")),
            PathType::Windows
        );
        assert_eq!(
            detect_path_type(Path::new(r"D:\Data\file.txt")),
            PathType::Windows
        );
        assert_eq!(
            detect_path_type(Path::new(r"Z:\Network\Share")),
            PathType::Windows
        );

        // Forward slashes still count as Windows if drive letter present
        assert_eq!(
            detect_path_type(Path::new("C:/Windows/System32")),
            PathType::Windows
        );
    }

    #[test]
    fn test_detect_unc_paths() {
        assert_eq!(
            detect_path_type(Path::new(r"\\server\share\file.txt")),
            PathType::Windows
        );
        assert_eq!(
            detect_path_type(Path::new("//server/share/file.txt")),
            PathType::Windows
        );
    }

    #[test]
    fn test_detect_wsl_paths() {
        assert_eq!(
            detect_path_type(Path::new("/mnt/c/Users/david")),
            PathType::Wsl
        );
        assert_eq!(detect_path_type(Path::new("/mnt/d/Data")), PathType::Wsl);
        assert_eq!(detect_path_type(Path::new("/mnt/z/")), PathType::Wsl);
        assert_eq!(detect_path_type(Path::new("/mnt/c")), PathType::Wsl);
    }

    #[test]
    fn test_detect_unix_paths() {
        assert_eq!(
            detect_path_type(Path::new("/home/david/file.txt")),
            PathType::Unix
        );
        assert_eq!(detect_path_type(Path::new("/usr/bin/bash")), PathType::Unix);
        assert_eq!(detect_path_type(Path::new("/etc/hosts")), PathType::Unix);
        assert_eq!(detect_path_type(Path::new("/")), PathType::Unix);
    }

    #[test]
    fn test_detect_relative_paths() {
        assert_eq!(
            detect_path_type(Path::new("relative/path/file.txt")),
            PathType::Unix
        );
        assert_eq!(detect_path_type(Path::new("file.txt")), PathType::Unix);
        assert_eq!(detect_path_type(Path::new("./file.txt")), PathType::Unix);
        assert_eq!(
            detect_path_type(Path::new("../parent/file.txt")),
            PathType::Unix
        );
    }

    #[test]
    fn test_to_wsl_path_basic() {
        let path = Path::new(r"C:\Users\david\file.txt");
        let wsl = to_wsl_path(path);
        assert_eq!(wsl.to_str().unwrap(), "/mnt/c/Users/david/file.txt");

        let path = Path::new(r"D:\Data\project");
        let wsl = to_wsl_path(path);
        assert_eq!(wsl.to_str().unwrap(), "/mnt/d/Data/project");
    }

    #[test]
    fn test_to_wsl_path_with_spaces() {
        let path = Path::new(r"C:\Program Files\My Application\file.txt");
        let wsl = to_wsl_path(path);
        assert_eq!(
            wsl.to_str().unwrap(),
            "/mnt/c/Program Files/My Application/file.txt"
        );
    }

    #[test]
    fn test_to_wsl_path_forward_slashes() {
        let path = Path::new("C:/Users/david/file.txt");
        let wsl = to_wsl_path(path);
        assert_eq!(wsl.to_str().unwrap(), "/mnt/c/Users/david/file.txt");
    }

    #[test]
    fn test_to_wsl_path_drive_root() {
        let path = Path::new(r"C:\");
        let wsl = to_wsl_path(path);
        assert_eq!(wsl.to_str().unwrap(), "/mnt/c/");

        let path = Path::new("D:");
        let wsl = to_wsl_path(path);
        assert!(wsl.to_str().unwrap().starts_with("/mnt/d"));
    }

    #[test]
    fn test_to_wsl_path_unc() {
        let path = Path::new(r"\\server\share\file.txt");
        let wsl = to_wsl_path(path);
        assert_eq!(wsl.to_str().unwrap(), "//server/share/file.txt");
    }

    #[test]
    fn test_to_wsl_path_idempotent() {
        let path = Path::new("/mnt/c/Users/david/file.txt");
        let wsl = to_wsl_path(path);
        assert_eq!(wsl.to_str().unwrap(), "/mnt/c/Users/david/file.txt");

        let path = Path::new("/home/user/file.txt");
        let wsl = to_wsl_path(path);
        assert_eq!(wsl.to_str().unwrap(), "/home/user/file.txt");
    }

    #[test]
    fn test_to_windows_path_basic() {
        let path = Path::new("/mnt/c/Users/david/file.txt");
        let win = to_windows_path(path);
        assert_eq!(win.to_str().unwrap(), r"C:\Users\david\file.txt");

        let path = Path::new("/mnt/d/Data/project");
        let win = to_windows_path(path);
        assert_eq!(win.to_str().unwrap(), r"D:\Data\project");
    }

    #[test]
    fn test_to_windows_path_with_spaces() {
        let path = Path::new("/mnt/c/Program Files/My Application/file.txt");
        let win = to_windows_path(path);
        assert_eq!(
            win.to_str().unwrap(),
            r"C:\Program Files\My Application\file.txt"
        );
    }

    #[test]
    fn test_to_windows_path_drive_root() {
        let path = Path::new("/mnt/c/");
        let win = to_windows_path(path);
        assert_eq!(win.to_str().unwrap(), r"C:\");

        let path = Path::new("/mnt/c");
        let win = to_windows_path(path);
        assert_eq!(win.to_str().unwrap(), r"C:\");
    }

    #[test]
    fn test_to_windows_path_unix_to_unc() {
        let path = Path::new("/home/david/file.txt");
        let win = to_windows_path(path);
        assert_eq!(
            win.to_str().unwrap(),
            r"\\wsl.localhost\Ubuntu\home\david\file.txt"
        );

        let path = Path::new("/usr/bin/bash");
        let win = to_windows_path(path);
        assert_eq!(
            win.to_str().unwrap(),
            r"\\wsl.localhost\Ubuntu\usr\bin\bash"
        );
    }

    #[test]
    fn test_to_windows_path_idempotent() {
        let path = Path::new(r"C:\Users\david\file.txt");
        let win = to_windows_path(path);
        assert_eq!(win.to_str().unwrap(), r"C:\Users\david\file.txt");

        let path = Path::new(r"\\server\share\file.txt");
        let win = to_windows_path(path);
        assert_eq!(win.to_str().unwrap(), r"\\server\share\file.txt");
    }

    #[test]
    fn test_roundtrip_conversion() {
        // Windows → WSL → Windows
        let original = Path::new(r"C:\Users\david\Documents\file.txt");
        let wsl = to_wsl_path(original);
        let back = to_windows_path(&wsl);
        assert_eq!(back.to_str().unwrap(), r"C:\Users\david\Documents\file.txt");

        // WSL → Windows → WSL
        let original = Path::new("/mnt/d/Projects/rust/src/main.rs");
        let win = to_windows_path(original);
        let back = to_wsl_path(&win);
        assert_eq!(back.to_str().unwrap(), "/mnt/d/Projects/rust/src/main.rs");
    }

    #[test]
    fn test_special_characters() {
        // Paths with special characters (but not invalid ones)
        let path = Path::new(r"C:\Users\david\My-Project_v2.0\file (1).txt");
        let wsl = to_wsl_path(path);
        assert_eq!(
            wsl.to_str().unwrap(),
            "/mnt/c/Users/david/My-Project_v2.0/file (1).txt"
        );

        let back = to_windows_path(&wsl);
        assert_eq!(
            back.to_str().unwrap(),
            r"C:\Users\david\My-Project_v2.0\file (1).txt"
        );
    }

    #[test]
    fn test_normalize_path_windows() {
        #[cfg(windows)]
        {
            let path = Path::new("/mnt/c/Users/david/file.txt");
            let normalized = normalize_path(path);
            assert_eq!(normalized.to_str().unwrap(), r"C:\Users\david\file.txt");

            let path = Path::new(r"C:\Users\david\file.txt");
            let normalized = normalize_path(path);
            assert_eq!(normalized.to_str().unwrap(), r"C:\Users\david\file.txt");
        }
    }

    #[test]
    fn test_normalize_path_unix() {
        #[cfg(not(windows))]
        {
            let path = Path::new(r"C:\Users\david\file.txt");
            let normalized = normalize_path(path);
            assert_eq!(normalized.to_str().unwrap(), "/mnt/c/Users/david/file.txt");

            let path = Path::new("/mnt/c/Users/david/file.txt");
            let normalized = normalize_path(path);
            assert_eq!(normalized.to_str().unwrap(), "/mnt/c/Users/david/file.txt");
        }
    }

    #[test]
    fn test_edge_case_empty_path() {
        let path = Path::new("");
        assert_eq!(detect_path_type(path), PathType::Unix);

        let wsl = to_wsl_path(path);
        assert_eq!(wsl.to_str().unwrap(), "");

        let win = to_windows_path(path);
        assert_eq!(win.to_str().unwrap(), "");
    }

    #[test]
    fn test_edge_case_single_char() {
        let path = Path::new("a");
        assert_eq!(detect_path_type(path), PathType::Unix);
    }

    #[test]
    fn test_case_sensitivity() {
        // Windows drive letters should be case-insensitive
        let path_lower = Path::new(r"c:\Users\david");
        let path_upper = Path::new(r"C:\Users\david");

        assert_eq!(detect_path_type(path_lower), PathType::Windows);
        assert_eq!(detect_path_type(path_upper), PathType::Windows);

        let wsl_lower = to_wsl_path(path_lower);
        let wsl_upper = to_wsl_path(path_upper);

        // Both should convert to lowercase drive letter in WSL
        assert_eq!(wsl_lower.to_str().unwrap(), "/mnt/c/Users/david");
        assert_eq!(wsl_upper.to_str().unwrap(), "/mnt/c/Users/david");
    }

    #[test]
    fn test_mixed_slashes() {
        // Windows paths with mixed slashes
        let path = Path::new(r"C:\Users/david\Documents/file.txt");
        let wsl = to_wsl_path(path);
        assert_eq!(
            wsl.to_str().unwrap(),
            "/mnt/c/Users/david/Documents/file.txt"
        );
    }
}
