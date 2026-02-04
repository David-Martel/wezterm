//! Example demonstrating WSL path translation utilities.
//!
//! Run with: cargo run --example path_translation

use std::path::Path;
use wezterm_fs_explorer::path_utils::{
    detect_path_type, normalize_path, to_windows_path, to_wsl_path,
};

fn main() {
    println!("=== WSL Path Translation Examples ===\n");

    // Example 1: Convert Windows path to WSL
    println!("1. Windows to WSL:");
    let win_path = Path::new(r"C:\Users\david\Documents\project");
    let wsl_path = to_wsl_path(win_path);
    println!("   Windows: {}", win_path.display());
    println!("   WSL:     {}\n", wsl_path.display());

    // Example 2: Convert WSL path to Windows
    println!("2. WSL to Windows:");
    let wsl_path = Path::new("/mnt/d/Data/file.txt");
    let win_path = to_windows_path(wsl_path);
    println!("   WSL:     {}", wsl_path.display());
    println!("   Windows: {}\n", win_path.display());

    // Example 3: Detect path types
    println!("3. Path Type Detection:");
    let paths = vec![
        r"C:\Windows\System32",
        "/mnt/c/Users/david",
        "/home/david/.config",
        r"\\server\share\file.txt",
        "relative/path/file.txt",
    ];

    for path_str in paths {
        let path = Path::new(path_str);
        let path_type = detect_path_type(path);
        println!("   {} → {:?}", path_str, path_type);
    }
    println!();

    // Example 4: Normalize paths to current platform
    println!("4. Platform Normalization:");
    let mixed_paths = vec![
        r"C:\Users\david\file.txt",
        "/mnt/c/Program Files/App",
        "/home/david/project",
    ];

    for path_str in mixed_paths {
        let path = Path::new(path_str);
        let normalized = normalize_path(path);
        println!("   {} → {}", path_str, normalized.display());
    }
    println!();

    // Example 5: Roundtrip conversions
    println!("5. Roundtrip Conversion:");
    let original = Path::new(r"C:\Users\david\Documents\test.txt");
    let to_wsl = to_wsl_path(original);
    let back_to_win = to_windows_path(&to_wsl);
    println!("   Original: {}", original.display());
    println!("   To WSL:   {}", to_wsl.display());
    println!("   Back:     {}", back_to_win.display());
    println!("   Match:    {}", original == back_to_win);
    println!();

    // Example 6: Edge cases
    println!("6. Edge Cases:");
    let edge_cases = vec![
        (r"C:\", "Drive root with backslash"),
        ("D:", "Drive root without backslash"),
        ("/mnt/c", "WSL mount point only"),
        ("/mnt/c/", "WSL mount point with trailing slash"),
        (r"C:\Program Files\My App", "Path with spaces"),
        (
            r"C:\My-Project_v2.0\file (1).txt",
            "Path with special characters",
        ),
    ];

    for (path_str, description) in edge_cases {
        let path = Path::new(path_str);
        let converted = to_wsl_path(path);
        println!("   {}", description);
        println!("     {} → {}\n", path_str, converted.display());
    }
}
