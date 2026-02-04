//! Shell detection and command translation for cross-platform support.
//!
//! This module provides utilities to detect the current shell environment
//! and translate commands between different shell syntaxes, particularly
//! for Windows environments (PowerShell, CMD, Git Bash, WSL).

// Library module - items are exported for external consumers
#![allow(dead_code)]

use std::env;
use std::path::Path;
use std::process::{Command, Output};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ShellError {
    #[error("Failed to execute command: {0}")]
    ExecutionFailed(#[from] std::io::Error),

    #[error("Command failed with exit code {code}: {stderr}")]
    CommandFailed { code: i32, stderr: String },

    #[error("Shell detection failed: {0}")]
    DetectionFailed(String),

    #[error("Unsupported shell operation: {0}")]
    UnsupportedOperation(String),
}

/// Represents different shell environments with their specific syntax requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // PowerShell is the actual product name
pub enum Shell {
    /// PowerShell (pwsh.exe or powershell.exe)
    PowerShell,
    /// Git Bash (MINGW64/MSYS)
    GitBash,
    /// WSL Bash (Windows Subsystem for Linux)
    WslBash,
    /// Windows Command Prompt
    Cmd,
    /// Unknown or unsupported shell
    Unknown,
}

impl Shell {
    /// Detect the current shell environment based on environment variables.
    ///
    /// Detection order:
    /// 1. Check SHELL environment variable
    /// 2. Check MSYSTEM (Git Bash)
    /// 3. Check WSL_DISTRO_NAME (WSL)
    /// 4. Check PSModulePath (PowerShell)
    /// 5. Default to CMD on Windows, Unknown otherwise
    pub fn detect() -> Self {
        // Check SHELL variable (Unix-like)
        if let Ok(shell) = env::var("SHELL") {
            if shell.contains("bash") {
                // Distinguish between WSL and Git Bash
                if env::var("WSL_DISTRO_NAME").is_ok() {
                    return Shell::WslBash;
                }
                if env::var("MSYSTEM").is_ok() {
                    return Shell::GitBash;
                }
                return Shell::GitBash; // Default bash to Git Bash on Windows
            }
        }

        // Check for Git Bash (MSYSTEM environment)
        if env::var("MSYSTEM").is_ok() {
            return Shell::GitBash;
        }

        // Check for WSL
        if env::var("WSL_DISTRO_NAME").is_ok() {
            return Shell::WslBash;
        }

        // Check for PowerShell (PSModulePath is unique to PowerShell)
        if let Ok(ps_module_path) = env::var("PSModulePath") {
            if !ps_module_path.is_empty() {
                return Shell::PowerShell;
            }
        }

        // On Windows, default to CMD
        #[cfg(target_os = "windows")]
        {
            Shell::Cmd
        }

        #[cfg(not(target_os = "windows"))]
        {
            Shell::Unknown
        }
    }

    /// Translate a command from generic syntax to shell-specific syntax.
    ///
    /// Handles:
    /// - Path separators
    /// - Quoting conventions
    /// - Variable expansion
    /// - Command chaining
    pub fn translate(&self, cmd: &str) -> String {
        match self {
            Shell::PowerShell => Self::translate_to_powershell(cmd),
            Shell::GitBash | Shell::WslBash => Self::translate_to_bash(cmd),
            Shell::Cmd => Self::translate_to_cmd(cmd),
            Shell::Unknown => cmd.to_string(),
        }
    }

    /// Translate path separators in a command for the target shell.
    pub fn translate_path(&self, path: &str) -> String {
        match self {
            Shell::PowerShell | Shell::Cmd => {
                // Windows prefers backslashes, but accepts forward slashes
                path.replace('/', "\\")
            }
            Shell::GitBash | Shell::WslBash => {
                // Unix-like prefers forward slashes
                path.replace('\\', "/")
            }
            Shell::Unknown => path.to_string(),
        }
    }

    /// Translate a command to PowerShell syntax.
    fn translate_to_powershell(cmd: &str) -> String {
        let mut result = cmd.to_string();

        // Replace Unix variable syntax with PowerShell
        result = result.replace("$VAR", "$env:VAR");

        // Replace command chaining
        result = result.replace(" && ", " ; ");

        // Replace single quotes with double quotes for consistency
        // (PowerShell prefers double quotes for expandable strings)
        if result.contains('\'') && !result.contains('"') {
            result = result.replace('\'', "\"");
        }

        result
    }

    /// Translate a command to Bash syntax (Git Bash or WSL).
    fn translate_to_bash(cmd: &str) -> String {
        let mut result = cmd.to_string();

        // Replace PowerShell variable syntax with Bash
        result = result.replace("$env:", "$");

        // PowerShell's -and operator to Bash &&
        result = result.replace(" -and ", " && ");

        // Replace path separators
        result = result.replace('\\', "/");

        result
    }

    /// Translate a command to CMD syntax.
    fn translate_to_cmd(cmd: &str) -> String {
        let mut result = cmd.to_string();

        // Replace Unix command chaining with CMD syntax
        result = result.replace(" && ", " & ");

        // Replace Unix variable syntax with CMD syntax
        result = result.replace("$VAR", "%VAR%");

        // Replace path separators
        result = result.replace('/', "\\");

        result
    }

    /// Get the shell executable path.
    pub fn executable(&self) -> &'static str {
        match self {
            Shell::PowerShell => {
                // Prefer pwsh (PowerShell Core) if available
                if which::which("pwsh").is_ok() {
                    "pwsh"
                } else {
                    "powershell"
                }
            }
            Shell::GitBash => "bash",
            Shell::WslBash => "wsl",
            Shell::Cmd => "cmd",
            Shell::Unknown => "sh",
        }
    }

    /// Get the command argument flag for the shell.
    pub fn command_flag(&self) -> &'static str {
        match self {
            Shell::PowerShell => "-Command",
            Shell::GitBash | Shell::WslBash => "-c",
            Shell::Cmd => "/C",
            Shell::Unknown => "-c",
        }
    }

    /// Execute a command in the detected shell environment.
    pub fn execute(&self, cmd: &str) -> Result<Output, ShellError> {
        let translated = self.translate(cmd);
        let output = Command::new(self.executable())
            .arg(self.command_flag())
            .arg(&translated)
            .output()?;

        if !output.status.success() {
            let code = output.status.code().unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(ShellError::CommandFailed { code, stderr });
        }

        Ok(output)
    }

    /// Quote a string for safe use in the shell.
    pub fn quote(&self, s: &str) -> String {
        match self {
            Shell::PowerShell => {
                // PowerShell uses double quotes and escapes with backtick
                if s.contains(' ') || s.contains('"') || s.contains('$') {
                    format!("\"{}\"", s.replace('"', "`\"").replace('$', "`$"))
                } else {
                    s.to_string()
                }
            }
            Shell::GitBash | Shell::WslBash => {
                // Bash uses single quotes for literal strings
                if s.contains(' ') || s.contains('\'') || s.contains('"') {
                    format!("'{}'", s.replace('\'', "'\\''"))
                } else {
                    s.to_string()
                }
            }
            Shell::Cmd => {
                // CMD uses double quotes and doesn't have robust escaping
                if s.contains(' ') {
                    format!("\"{}\"", s)
                } else {
                    s.to_string()
                }
            }
            Shell::Unknown => s.to_string(),
        }
    }

    /// Check if a path exists using the shell.
    pub fn path_exists(&self, path: &str) -> Result<bool, ShellError> {
        let translated_path = self.translate_path(path);
        let check_cmd = match self {
            Shell::PowerShell => format!("Test-Path '{}'", translated_path),
            Shell::GitBash | Shell::WslBash => format!("test -e '{}'", translated_path),
            Shell::Cmd => format!("if exist \"{}\" echo 1", translated_path),
            Shell::Unknown => return Ok(Path::new(path).exists()),
        };

        match self.execute(&check_cmd) {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(stdout.trim() == "True" || stdout.trim() == "1")
            }
            Err(_) => Ok(false),
        }
    }
}

/// Detect the current shell and return a Shell instance.
pub fn detect_shell() -> Shell {
    Shell::detect()
}

/// Translate a command to the target shell's syntax.
pub fn translate_command(cmd: &str, target: Shell) -> String {
    target.translate(cmd)
}

/// Translate path separators in a command for the target shell.
pub fn translate_path_in_command(cmd: &str, shell: Shell) -> String {
    shell.translate_path(cmd)
}

/// Execute a command with automatic shell detection.
pub fn execute_command(cmd: &str) -> Result<Output, ShellError> {
    let shell = detect_shell();
    shell.execute(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_detection() {
        // Shell detection should not panic
        let shell = Shell::detect();
        assert!(matches!(
            shell,
            Shell::PowerShell | Shell::GitBash | Shell::WslBash | Shell::Cmd | Shell::Unknown
        ));
    }

    #[test]
    fn test_powershell_translation() {
        let shell = Shell::PowerShell;

        // Variable expansion
        let cmd = "echo $VAR";
        let translated = shell.translate(cmd);
        assert!(translated.contains("$env:VAR"));

        // Command chaining
        let cmd = "cd dir && ls";
        let translated = shell.translate(cmd);
        assert!(translated.contains(" ; "));
    }

    #[test]
    fn test_bash_translation() {
        let shell = Shell::GitBash;

        // PowerShell variable to Bash
        let cmd = "echo $env:PATH";
        let translated = shell.translate(cmd);
        assert!(translated.contains("$PATH"));
        assert!(!translated.contains("$env:"));

        // Path separators
        let cmd = "cd C:\\Users\\test";
        let translated = shell.translate(cmd);
        assert!(translated.contains("C:/Users/test"));
    }

    #[test]
    fn test_cmd_translation() {
        let shell = Shell::Cmd;

        // Command chaining
        let cmd = "cd dir && dir";
        let translated = shell.translate(cmd);
        assert!(translated.contains(" & "));

        // Path separators
        let cmd = "cd /Users/test";
        let translated = shell.translate(cmd);
        assert!(translated.contains("\\Users\\test"));
    }

    #[test]
    fn test_path_translation() {
        // Windows shells
        let ps = Shell::PowerShell;
        assert_eq!(ps.translate_path("C:/Users/test"), "C:\\Users\\test");

        let cmd = Shell::Cmd;
        assert_eq!(cmd.translate_path("path/to/file"), "path\\to\\file");

        // Unix-like shells
        let bash = Shell::GitBash;
        assert_eq!(bash.translate_path("C:\\Users\\test"), "C:/Users/test");

        let wsl = Shell::WslBash;
        assert_eq!(wsl.translate_path("path\\to\\file"), "path/to/file");
    }

    #[test]
    fn test_quoting() {
        let ps = Shell::PowerShell;
        assert_eq!(ps.quote("simple"), "simple");
        assert_eq!(ps.quote("with space"), "\"with space\"");
        assert_eq!(ps.quote("with \"quote\""), "\"with `\"quote`\"\"");

        let bash = Shell::GitBash;
        assert_eq!(bash.quote("simple"), "simple");
        assert_eq!(bash.quote("with space"), "'with space'");
        assert_eq!(bash.quote("with 'quote'"), "'with '\\''quote'\\'''");

        let cmd = Shell::Cmd;
        assert_eq!(cmd.quote("simple"), "simple");
        assert_eq!(cmd.quote("with space"), "\"with space\"");
    }

    #[test]
    fn test_executable_and_flags() {
        let ps = Shell::PowerShell;
        assert!(ps.executable() == "pwsh" || ps.executable() == "powershell");
        assert_eq!(ps.command_flag(), "-Command");

        let bash = Shell::GitBash;
        assert_eq!(bash.executable(), "bash");
        assert_eq!(bash.command_flag(), "-c");

        let cmd = Shell::Cmd;
        assert_eq!(cmd.executable(), "cmd");
        assert_eq!(cmd.command_flag(), "/C");

        let wsl = Shell::WslBash;
        assert_eq!(wsl.executable(), "wsl");
        assert_eq!(wsl.command_flag(), "-c");
    }

    #[test]
    fn test_shell_equality() {
        assert_eq!(Shell::PowerShell, Shell::PowerShell);
        assert_ne!(Shell::PowerShell, Shell::GitBash);
        assert_ne!(Shell::GitBash, Shell::WslBash);
    }

    #[test]
    fn test_debug_trait() {
        let shell = Shell::PowerShell;
        let debug_str = format!("{:?}", shell);
        assert!(debug_str.contains("PowerShell"));
    }

    #[test]
    fn test_clone_and_copy() {
        let shell1 = Shell::PowerShell;
        let shell2 = shell1; // Copy
        assert_eq!(shell1, shell2);

        let shell3 = shell1; // Also Copy
        assert_eq!(shell1, shell3);
    }

    #[test]
    fn test_complex_command_translation() {
        let ps = Shell::PowerShell;
        let cmd = "cd $HOME && echo $VAR";
        let translated = ps.translate(cmd);
        assert!(translated.contains("$env:"));
        assert!(translated.contains(" ; "));

        let bash = Shell::GitBash;
        let cmd = "cd $env:HOME -and test";
        let translated = bash.translate(cmd);
        assert!(translated.contains("$HOME"));
        assert!(translated.contains(" && "));
    }

    #[test]
    fn test_error_types() {
        // Test that error types implement Debug
        let err = ShellError::DetectionFailed("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("DetectionFailed"));

        // Test Display trait (from thiserror)
        let err_msg = format!("{}", err);
        assert!(err_msg.contains("Shell detection failed"));
    }
}
