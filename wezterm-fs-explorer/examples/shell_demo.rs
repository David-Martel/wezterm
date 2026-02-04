//! Demonstrates shell detection and command translation capabilities.

use wezterm_fs_explorer::shell::{detect_shell, Shell};

fn main() {
    // Detect the current shell environment
    let shell = detect_shell();
    println!("Detected shell: {:?}", shell);
    println!("Executable: {}", shell.executable());
    println!("Command flag: {}", shell.command_flag());
    println!();

    // Demonstrate command translation
    let commands = vec![
        "cd $HOME && ls -la",
        "echo $VAR",
        "cd /path/to/dir && pwd",
    ];

    println!("Command translations for {:?}:", shell);
    for cmd in commands {
        let translated = shell.translate(cmd);
        println!("  Original:   {}", cmd);
        println!("  Translated: {}", translated);
        println!();
    }

    // Demonstrate path translation
    let paths = vec![
        "C:\\Users\\david\\file.txt",
        "/home/david/file.txt",
        "relative/path/file.txt",
    ];

    println!("Path translations for {:?}:", shell);
    for path in paths {
        let translated = shell.translate_path(path);
        println!("  Original:   {}", path);
        println!("  Translated: {}", translated);
        println!();
    }

    // Demonstrate quoting
    let strings = vec![
        "simple",
        "with space",
        "with \"quotes\"",
        "with $variable",
    ];

    println!("String quoting for {:?}:", shell);
    for s in strings {
        let quoted = shell.quote(s);
        println!("  Original: {}", s);
        println!("  Quoted:   {}", quoted);
        println!();
    }

    // Demonstrate translation between shells
    println!("Cross-shell translations:");
    let cmd = "cd $HOME && echo $VAR";

    for target in &[Shell::PowerShell, Shell::GitBash, Shell::Cmd] {
        let translated = target.translate(cmd);
        println!("  {:?}: {}", target, translated);
    }
}
