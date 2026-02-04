use crate::git::{FileStatus, GitInfo};
use crate::watcher::WatchEvent;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Pretty,
    Events,
    Summary,
}

/// Error type for OutputFormat parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutputFormatError(String);

impl std::fmt::Display for ParseOutputFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ParseOutputFormatError {}

impl FromStr for OutputFormat {
    type Err = ParseOutputFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "pretty" => Ok(Self::Pretty),
            "events" => Ok(Self::Events),
            "summary" => Ok(Self::Summary),
            _ => Err(ParseOutputFormatError(format!("Unknown output format: {}", s))),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonOutput {
    pub event_type: String,
    pub path: Option<PathBuf>,
    pub from_path: Option<PathBuf>,
    pub to_path: Option<PathBuf>,
    pub git_status: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonSummary {
    pub git_branch: Option<String>,
    pub git_ahead: Option<usize>,
    pub git_behind: Option<usize>,
    pub has_conflicts: bool,
    pub modified_files: usize,
    pub untracked_files: usize,
    pub staged_files: usize,
    pub total_files: usize,
}

pub struct OutputFormatter {
    format: OutputFormat,
}

impl OutputFormatter {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    pub fn format_event(&self, event: &WatchEvent, git_status: Option<&FileStatus>) -> String {
        match self.format {
            OutputFormat::Json => self.format_json(event, git_status),
            OutputFormat::Pretty => self.format_pretty(event, git_status),
            OutputFormat::Events => self.format_events(event, git_status),
            OutputFormat::Summary => String::new(), // Summary doesn't output per-event
        }
    }

    pub fn format_git_info(&self, info: &GitInfo) -> String {
        match self.format {
            OutputFormat::Json => self.format_git_json(info),
            OutputFormat::Pretty => self.format_git_pretty(info),
            OutputFormat::Summary => self.format_git_summary(info),
            OutputFormat::Events => String::new(), // Events mode doesn't show git info
        }
    }

    fn format_json(&self, event: &WatchEvent, git_status: Option<&FileStatus>) -> String {
        let output = match event {
            WatchEvent::Created(path) => JsonOutput {
                event_type: "created".to_string(),
                path: Some(path.clone()),
                from_path: None,
                to_path: None,
                git_status: git_status.map(|s| s.to_short_str().to_string()),
                timestamp: Self::current_timestamp(),
            },
            WatchEvent::Modified(path) => JsonOutput {
                event_type: "modified".to_string(),
                path: Some(path.clone()),
                from_path: None,
                to_path: None,
                git_status: git_status.map(|s| s.to_short_str().to_string()),
                timestamp: Self::current_timestamp(),
            },
            WatchEvent::Deleted(path) => JsonOutput {
                event_type: "deleted".to_string(),
                path: Some(path.clone()),
                from_path: None,
                to_path: None,
                git_status: git_status.map(|s| s.to_short_str().to_string()),
                timestamp: Self::current_timestamp(),
            },
            WatchEvent::Renamed { from, to } => JsonOutput {
                event_type: "renamed".to_string(),
                path: None,
                from_path: Some(from.clone()),
                to_path: Some(to.clone()),
                git_status: git_status.map(|s| s.to_short_str().to_string()),
                timestamp: Self::current_timestamp(),
            },
            WatchEvent::Error(_msg) => JsonOutput {
                event_type: "error".to_string(),
                path: None,
                from_path: None,
                to_path: None,
                git_status: None,
                timestamp: Self::current_timestamp(),
            },
        };

        serde_json::to_string(&output).unwrap_or_default()
    }

    fn format_pretty(&self, event: &WatchEvent, git_status: Option<&FileStatus>) -> String {
        let git_indicator = if let Some(status) = git_status {
            format!("[{}] ", status.to_colored_str())
        } else {
            String::new()
        };

        match event {
            WatchEvent::Created(path) => {
                format!(
                    "{}{} {}",
                    git_indicator,
                    "CREATED".green().bold(),
                    path.display()
                )
            }
            WatchEvent::Modified(path) => {
                format!(
                    "{}{} {}",
                    git_indicator,
                    "MODIFIED".yellow().bold(),
                    path.display()
                )
            }
            WatchEvent::Deleted(path) => {
                format!(
                    "{}{} {}",
                    git_indicator,
                    "DELETED".red().bold(),
                    path.display()
                )
            }
            WatchEvent::Renamed { from, to } => {
                format!(
                    "{}{} {} -> {}",
                    git_indicator,
                    "RENAMED".blue().bold(),
                    from.display(),
                    to.display()
                )
            }
            WatchEvent::Error(msg) => {
                format!("{} {}", "ERROR".red().bold(), msg)
            }
        }
    }

    fn format_events(&self, event: &WatchEvent, git_status: Option<&FileStatus>) -> String {
        let git_indicator = if let Some(status) = git_status {
            status.to_short_str()
        } else {
            " "
        };

        match event {
            WatchEvent::Created(path) => {
                format!("{} + {}", git_indicator, path.display())
            }
            WatchEvent::Modified(path) => {
                format!("{} ~ {}", git_indicator, path.display())
            }
            WatchEvent::Deleted(path) => {
                format!("{} - {}", git_indicator, path.display())
            }
            WatchEvent::Renamed { from, to } => {
                format!("{} R {} -> {}", git_indicator, from.display(), to.display())
            }
            WatchEvent::Error(msg) => {
                format!("! {}", msg)
            }
        }
    }

    fn format_git_json(&self, info: &GitInfo) -> String {
        let summary = JsonSummary {
            git_branch: Some(info.branch.clone()),
            git_ahead: Some(info.ahead),
            git_behind: Some(info.behind),
            has_conflicts: info.has_conflicts,
            modified_files: info
                .file_statuses
                .values()
                .filter(|s| **s == FileStatus::Modified)
                .count(),
            untracked_files: info
                .file_statuses
                .values()
                .filter(|s| **s == FileStatus::Untracked)
                .count(),
            staged_files: info
                .file_statuses
                .values()
                .filter(|s| **s == FileStatus::Staged)
                .count(),
            total_files: info.file_statuses.len(),
        };

        serde_json::to_string(&summary).unwrap_or_default()
    }

    fn format_git_pretty(&self, info: &GitInfo) -> String {
        let mut output = String::new();

        // Branch info
        output.push_str(&format!(
            "{} {}\n",
            "Branch:".cyan().bold(),
            info.branch.bright_white()
        ));

        // Ahead/Behind
        if info.ahead > 0 || info.behind > 0 {
            output.push_str(&format!(
                "{} {} ahead, {} behind\n",
                "Status:".cyan().bold(),
                format!("{}", info.ahead).green(),
                format!("{}", info.behind).red()
            ));
        }

        // Conflicts
        if info.has_conflicts {
            output.push_str(&format!("{}\n", "CONFLICTS DETECTED".red().bold()));
        }

        // File counts
        let modified = info
            .file_statuses
            .values()
            .filter(|s| **s == FileStatus::Modified)
            .count();
        let untracked = info
            .file_statuses
            .values()
            .filter(|s| **s == FileStatus::Untracked)
            .count();
        let staged = info
            .file_statuses
            .values()
            .filter(|s| **s == FileStatus::Staged)
            .count();

        output.push_str(&format!(
            "{} {} modified, {} staged, {} untracked\n",
            "Files:".cyan().bold(),
            modified,
            staged,
            untracked
        ));

        output
    }

    fn format_git_summary(&self, info: &GitInfo) -> String {
        let modified = info
            .file_statuses
            .values()
            .filter(|s| **s == FileStatus::Modified)
            .count();
        let untracked = info
            .file_statuses
            .values()
            .filter(|s| **s == FileStatus::Untracked)
            .count();
        let staged = info
            .file_statuses
            .values()
            .filter(|s| **s == FileStatus::Staged)
            .count();

        format!(
            "[{}] ↑{} ↓{} | M:{} S:{} U:{}{}",
            info.branch,
            info.ahead,
            info.behind,
            modified,
            staged,
            untracked,
            if info.has_conflicts {
                " [CONFLICT]"
            } else {
                ""
            }
        )
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ============================================
    // OutputFormat Unit Tests
    // ============================================

    #[test]
    fn test_output_format_from_str_valid() {
        assert_eq!(OutputFormat::from_str("json"), Ok(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("pretty"), Ok(OutputFormat::Pretty));
        assert_eq!(OutputFormat::from_str("events"), Ok(OutputFormat::Events));
        assert_eq!(OutputFormat::from_str("summary"), Ok(OutputFormat::Summary));
    }

    #[test]
    fn test_output_format_from_str_case_insensitive() {
        assert_eq!(OutputFormat::from_str("JSON"), Ok(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("Json"), Ok(OutputFormat::Json));
        assert_eq!(OutputFormat::from_str("PRETTY"), Ok(OutputFormat::Pretty));
        assert_eq!(OutputFormat::from_str("Pretty"), Ok(OutputFormat::Pretty));
        assert_eq!(OutputFormat::from_str("EVENTS"), Ok(OutputFormat::Events));
        assert_eq!(OutputFormat::from_str("SUMMARY"), Ok(OutputFormat::Summary));
    }

    #[test]
    fn test_output_format_from_str_invalid() {
        assert!(OutputFormat::from_str("invalid").is_err());
        assert!(OutputFormat::from_str("").is_err());
        assert!(OutputFormat::from_str("xml").is_err());
        assert!(OutputFormat::from_str("csv").is_err());
    }

    #[test]
    fn test_output_format_equality() {
        assert_eq!(OutputFormat::Json, OutputFormat::Json);
        assert_ne!(OutputFormat::Json, OutputFormat::Pretty);
        assert_ne!(OutputFormat::Events, OutputFormat::Summary);
    }

    #[test]
    fn test_output_format_copy() {
        let format = OutputFormat::Json;
        let copied = format;
        assert_eq!(format, copied);
    }

    // ============================================
    // OutputFormatter - JSON Format Tests
    // ============================================

    #[test]
    fn test_format_event_json_created() {
        let formatter = OutputFormatter::new(OutputFormat::Json);
        let event = WatchEvent::Created(PathBuf::from("test.txt"));
        let output = formatter.format_event(&event, None);

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["event_type"], "created");
        assert_eq!(parsed["path"], "test.txt");
        assert!(parsed["timestamp"].is_number());
    }

    #[test]
    fn test_format_event_json_modified() {
        let formatter = OutputFormatter::new(OutputFormat::Json);
        let event = WatchEvent::Modified(PathBuf::from("src/main.rs"));
        let output = formatter.format_event(&event, Some(&FileStatus::Modified));

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["event_type"], "modified");
        assert_eq!(parsed["git_status"], "M");
    }

    #[test]
    fn test_format_event_json_deleted() {
        let formatter = OutputFormatter::new(OutputFormat::Json);
        let event = WatchEvent::Deleted(PathBuf::from("old_file.txt"));
        let output = formatter.format_event(&event, None);

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["event_type"], "deleted");
        assert!(parsed["path"].as_str().unwrap().contains("old_file.txt"));
    }

    #[test]
    fn test_format_event_json_renamed() {
        let formatter = OutputFormatter::new(OutputFormat::Json);
        let event = WatchEvent::Renamed {
            from: PathBuf::from("old.txt"),
            to: PathBuf::from("new.txt"),
        };
        let output = formatter.format_event(&event, None);

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["event_type"], "renamed");
        assert!(parsed["from_path"].as_str().unwrap().contains("old.txt"));
        assert!(parsed["to_path"].as_str().unwrap().contains("new.txt"));
    }

    #[test]
    fn test_format_event_json_error() {
        let formatter = OutputFormatter::new(OutputFormat::Json);
        let event = WatchEvent::Error("Something went wrong".to_string());
        let output = formatter.format_event(&event, None);

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["event_type"], "error");
        assert!(parsed["git_status"].is_null());
    }

    #[test]
    fn test_format_event_json_with_git_status() {
        let formatter = OutputFormatter::new(OutputFormat::Json);
        let event = WatchEvent::Created(PathBuf::from("new_file.rs"));

        // Test various git statuses
        let statuses = vec![
            (FileStatus::Modified, "M"),
            (FileStatus::Added, "A"),
            (FileStatus::Deleted, "D"),
            (FileStatus::Untracked, "?"),
            (FileStatus::Staged, "S"),
        ];

        for (status, expected) in statuses {
            let output = formatter.format_event(&event, Some(&status));
            let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
            assert_eq!(parsed["git_status"], expected);
        }
    }

    // ============================================
    // OutputFormatter - Pretty Format Tests
    // ============================================

    #[test]
    fn test_format_event_pretty_created() {
        let formatter = OutputFormatter::new(OutputFormat::Pretty);
        let event = WatchEvent::Created(PathBuf::from("test.txt"));
        let output = formatter.format_event(&event, None);

        assert!(output.contains("CREATED"));
        assert!(output.contains("test.txt"));
    }

    #[test]
    fn test_format_event_pretty_modified() {
        let formatter = OutputFormatter::new(OutputFormat::Pretty);
        let event = WatchEvent::Modified(PathBuf::from("test.txt"));
        let output = formatter.format_event(&event, None);

        assert!(output.contains("MODIFIED"));
        assert!(output.contains("test.txt"));
    }

    #[test]
    fn test_format_event_pretty_deleted() {
        let formatter = OutputFormatter::new(OutputFormat::Pretty);
        let event = WatchEvent::Deleted(PathBuf::from("test.txt"));
        let output = formatter.format_event(&event, None);

        assert!(output.contains("DELETED"));
        assert!(output.contains("test.txt"));
    }

    #[test]
    fn test_format_event_pretty_renamed() {
        let formatter = OutputFormatter::new(OutputFormat::Pretty);
        let event = WatchEvent::Renamed {
            from: PathBuf::from("old.txt"),
            to: PathBuf::from("new.txt"),
        };
        let output = formatter.format_event(&event, None);

        assert!(output.contains("RENAMED"));
        assert!(output.contains("old.txt"));
        assert!(output.contains("new.txt"));
        assert!(output.contains("->"));
    }

    #[test]
    fn test_format_event_pretty_error() {
        let formatter = OutputFormatter::new(OutputFormat::Pretty);
        let event = WatchEvent::Error("Test error message".to_string());
        let output = formatter.format_event(&event, None);

        assert!(output.contains("ERROR"));
        assert!(output.contains("Test error message"));
    }

    #[test]
    fn test_format_event_pretty_with_git_status() {
        let formatter = OutputFormatter::new(OutputFormat::Pretty);
        let event = WatchEvent::Modified(PathBuf::from("file.rs"));
        let output = formatter.format_event(&event, Some(&FileStatus::Modified));

        // Should contain git indicator bracket
        assert!(output.contains("["));
        assert!(output.contains("]"));
    }

    // ============================================
    // OutputFormatter - Events Format Tests
    // ============================================

    #[test]
    fn test_format_event_events_created() {
        let formatter = OutputFormatter::new(OutputFormat::Events);
        let event = WatchEvent::Created(PathBuf::from("test.txt"));
        let output = formatter.format_event(&event, None);

        assert!(output.contains("+"));
        assert!(output.contains("test.txt"));
    }

    #[test]
    fn test_format_event_events_modified() {
        let formatter = OutputFormatter::new(OutputFormat::Events);
        let event = WatchEvent::Modified(PathBuf::from("test.txt"));
        let output = formatter.format_event(&event, None);

        assert!(output.contains("~"));
        assert!(output.contains("test.txt"));
    }

    #[test]
    fn test_format_event_events_deleted() {
        let formatter = OutputFormatter::new(OutputFormat::Events);
        let event = WatchEvent::Deleted(PathBuf::from("test.txt"));
        let output = formatter.format_event(&event, None);

        assert!(output.contains("-"));
        assert!(output.contains("test.txt"));
    }

    #[test]
    fn test_format_event_events_renamed() {
        let formatter = OutputFormatter::new(OutputFormat::Events);
        let event = WatchEvent::Renamed {
            from: PathBuf::from("old.txt"),
            to: PathBuf::from("new.txt"),
        };
        let output = formatter.format_event(&event, None);

        assert!(output.contains("R"));
        assert!(output.contains("old.txt"));
        assert!(output.contains("new.txt"));
    }

    #[test]
    fn test_format_event_events_error() {
        let formatter = OutputFormatter::new(OutputFormat::Events);
        let event = WatchEvent::Error("Error message".to_string());
        let output = formatter.format_event(&event, None);

        assert!(output.contains("!"));
        assert!(output.contains("Error message"));
    }

    #[test]
    fn test_format_event_events_with_git_status() {
        let formatter = OutputFormatter::new(OutputFormat::Events);
        let event = WatchEvent::Modified(PathBuf::from("file.rs"));
        let output = formatter.format_event(&event, Some(&FileStatus::Staged));

        assert!(output.contains("S"));
    }

    // ============================================
    // OutputFormatter - Summary Format Tests
    // ============================================

    #[test]
    fn test_format_event_summary_returns_empty() {
        let formatter = OutputFormatter::new(OutputFormat::Summary);
        let event = WatchEvent::Created(PathBuf::from("test.txt"));
        let output = formatter.format_event(&event, None);

        // Summary mode doesn't output per-event
        assert!(output.is_empty());
    }

    // ============================================
    // OutputFormatter - Git Info Tests
    // ============================================

    #[test]
    fn test_format_git_info_json() {
        let formatter = OutputFormatter::new(OutputFormat::Json);
        let mut statuses = HashMap::new();
        statuses.insert(PathBuf::from("file1.rs"), FileStatus::Modified);
        statuses.insert(PathBuf::from("file2.rs"), FileStatus::Untracked);
        statuses.insert(PathBuf::from("file3.rs"), FileStatus::Staged);

        let info = GitInfo {
            branch: "main".to_string(),
            ahead: 2,
            behind: 1,
            has_conflicts: false,
            file_statuses: statuses,
        };

        let output = formatter.format_git_info(&info);
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["git_branch"], "main");
        assert_eq!(parsed["git_ahead"], 2);
        assert_eq!(parsed["git_behind"], 1);
        assert_eq!(parsed["has_conflicts"], false);
        assert_eq!(parsed["modified_files"], 1);
        assert_eq!(parsed["untracked_files"], 1);
        assert_eq!(parsed["staged_files"], 1);
        assert_eq!(parsed["total_files"], 3);
    }

    #[test]
    fn test_format_git_info_pretty() {
        let formatter = OutputFormatter::new(OutputFormat::Pretty);
        let info = GitInfo {
            branch: "feature-branch".to_string(),
            ahead: 3,
            behind: 2,
            has_conflicts: true,
            file_statuses: HashMap::new(),
        };

        let output = formatter.format_git_info(&info);

        assert!(output.contains("feature-branch"));
        assert!(output.contains("3")); // ahead
        assert!(output.contains("2")); // behind
        assert!(output.contains("CONFLICTS"));
    }

    #[test]
    fn test_format_git_info_summary() {
        let formatter = OutputFormatter::new(OutputFormat::Summary);
        let mut statuses = HashMap::new();
        statuses.insert(PathBuf::from("mod.rs"), FileStatus::Modified);
        statuses.insert(PathBuf::from("new.rs"), FileStatus::Untracked);

        let info = GitInfo {
            branch: "develop".to_string(),
            ahead: 1,
            behind: 0,
            has_conflicts: false,
            file_statuses: statuses,
        };

        let output = formatter.format_git_info(&info);

        assert!(output.contains("[develop]"));
        assert!(output.contains("↑1"));
        assert!(output.contains("↓0"));
        assert!(output.contains("M:1"));
        assert!(output.contains("U:1"));
        assert!(!output.contains("[CONFLICT]"));
    }

    #[test]
    fn test_format_git_info_summary_with_conflicts() {
        let formatter = OutputFormatter::new(OutputFormat::Summary);
        let info = GitInfo {
            branch: "main".to_string(),
            ahead: 0,
            behind: 0,
            has_conflicts: true,
            file_statuses: HashMap::new(),
        };

        let output = formatter.format_git_info(&info);
        assert!(output.contains("[CONFLICT]"));
    }

    #[test]
    fn test_format_git_info_events_returns_empty() {
        let formatter = OutputFormatter::new(OutputFormat::Events);
        let info = GitInfo {
            branch: "main".to_string(),
            ahead: 0,
            behind: 0,
            has_conflicts: false,
            file_statuses: HashMap::new(),
        };

        let output = formatter.format_git_info(&info);
        assert!(output.is_empty());
    }

    // ============================================
    // JsonOutput Serialization Tests
    // ============================================

    #[test]
    fn test_json_output_serialization() {
        let output = JsonOutput {
            event_type: "created".to_string(),
            path: Some(PathBuf::from("test.txt")),
            from_path: None,
            to_path: None,
            git_status: Some("M".to_string()),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"event_type\":\"created\""));
        assert!(json.contains("\"timestamp\":1234567890"));
    }

    #[test]
    fn test_json_output_deserialization() {
        let json = r#"{"event_type":"modified","path":"test.rs","from_path":null,"to_path":null,"git_status":"M","timestamp":1000}"#;
        let output: JsonOutput = serde_json::from_str(json).unwrap();

        assert_eq!(output.event_type, "modified");
        assert_eq!(output.path, Some(PathBuf::from("test.rs")));
        assert_eq!(output.git_status, Some("M".to_string()));
    }

    #[test]
    fn test_json_summary_serialization() {
        let summary = JsonSummary {
            git_branch: Some("main".to_string()),
            git_ahead: Some(2),
            git_behind: Some(1),
            has_conflicts: false,
            modified_files: 3,
            untracked_files: 1,
            staged_files: 2,
            total_files: 6,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["git_branch"], "main");
        assert_eq!(parsed["total_files"], 6);
    }

    // ============================================
    // Timestamp Tests
    // ============================================

    #[test]
    fn test_current_timestamp_is_reasonable() {
        let ts = OutputFormatter::current_timestamp();
        // Timestamp should be after year 2024 (1704067200)
        assert!(ts > 1704067200);
        // And before year 2100
        assert!(ts < 4102444800);
    }
}
