use anyhow::Result;
use chrono::{DateTime, Local};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub file_type: FileType,
    pub size: u64,
    pub modified: SystemTime,
    pub permissions: String,
    pub is_hidden: bool,
}

impl FileEntry {
    pub fn from_path(path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path)?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let file_type = if metadata.is_dir() {
            FileType::Directory
        } else if metadata.file_type().is_symlink() {
            FileType::Symlink
        } else {
            FileType::File
        };

        let is_hidden = name.starts_with('.');

        Ok(Self {
            path: path.to_path_buf(),
            name,
            file_type,
            size: metadata.len(),
            modified: metadata.modified()?,
            permissions: Self::format_permissions(&metadata),
            is_hidden,
        })
    }

    pub fn read_directory(dir: &Path, show_hidden: bool) -> Result<Vec<Self>> {
        let mut entries = Vec::new();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Ok(file_entry) = Self::from_path(&path) {
                if show_hidden || !file_entry.is_hidden {
                    entries.push(file_entry);
                }
            }
        }

        // Sort: directories first, then by name
        entries.sort_by(|a, b| {
            match (&a.file_type, &b.file_type) {
                (FileType::Directory, FileType::Directory) => a.name.cmp(&b.name),
                (FileType::Directory, _) => std::cmp::Ordering::Less,
                (_, FileType::Directory) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        Ok(entries)
    }

    #[cfg(unix)]
    fn format_permissions(metadata: &Metadata) -> String {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        let user = triplet(mode, 0o100, 0o200, 0o400);
        let group = triplet(mode, 0o010, 0o020, 0o040);
        let other = triplet(mode, 0o001, 0o002, 0o004);
        format!("{}{}{}", user, group, other)
    }

    #[cfg(windows)]
    fn format_permissions(metadata: &Metadata) -> String {
        if metadata.permissions().readonly() {
            "r--".to_string()
        } else {
            "rw-".to_string()
        }
    }

    pub fn format_size(&self) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = self.size as f64;
        let mut unit_idx = 0;

        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }

        if unit_idx == 0 {
            format!("{} {}", size as u64, UNITS[unit_idx])
        } else {
            format!("{:.1} {}", size, UNITS[unit_idx])
        }
    }

    pub fn format_modified(&self) -> String {
        let datetime: DateTime<Local> = self.modified.into();
        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    pub fn extension(&self) -> Option<String> {
        self.path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
    }
}

#[cfg(unix)]
fn triplet(mode: u32, read: u32, write: u32, execute: u32) -> String {
    format!(
        "{}{}{}",
        if mode & read != 0 { "r" } else { "-" },
        if mode & write != 0 { "w" } else { "-" },
        if mode & execute != 0 { "x" } else { "-" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    // ============================================
    // FileType Tests
    // ============================================

    #[test]
    fn test_file_type_equality() {
        assert_eq!(FileType::File, FileType::File);
        assert_eq!(FileType::Directory, FileType::Directory);
        assert_eq!(FileType::Symlink, FileType::Symlink);
        assert_ne!(FileType::File, FileType::Directory);
    }

    #[test]
    fn test_file_type_clone() {
        let file_type = FileType::Directory;
        let cloned = file_type.clone();
        assert_eq!(file_type, cloned);
    }

    // ============================================
    // FileEntry::from_path Tests
    // ============================================

    #[test]
    fn test_from_path_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();

        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.file_type, FileType::File);
        assert!(!entry.is_hidden);
    }

    #[test]
    fn test_from_path_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("subdir");
        fs::create_dir(&dir_path).unwrap();

        let entry = FileEntry::from_path(&dir_path).unwrap();

        assert_eq!(entry.name, "subdir");
        assert_eq!(entry.file_type, FileType::Directory);
        assert!(!entry.is_hidden);
    }

    #[test]
    fn test_from_path_hidden_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".hidden");
        File::create(&file_path).unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();

        assert_eq!(entry.name, ".hidden");
        assert!(entry.is_hidden);
    }

    #[test]
    fn test_from_path_nonexistent() {
        let result = FileEntry::from_path(Path::new("/nonexistent/path/file.txt"));
        assert!(result.is_err());
    }

    // ============================================
    // FileEntry::read_directory Tests
    // ============================================

    #[test]
    fn test_read_directory_empty() {
        let temp_dir = TempDir::new().unwrap();
        let entries = FileEntry::read_directory(temp_dir.path(), false).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_read_directory_with_files() {
        let temp_dir = TempDir::new().unwrap();
        File::create(temp_dir.path().join("file1.txt")).unwrap();
        File::create(temp_dir.path().join("file2.txt")).unwrap();

        let entries = FileEntry::read_directory(temp_dir.path(), false).unwrap();

        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_read_directory_hidden_files_hidden() {
        let temp_dir = TempDir::new().unwrap();
        File::create(temp_dir.path().join("visible.txt")).unwrap();
        File::create(temp_dir.path().join(".hidden")).unwrap();

        let entries = FileEntry::read_directory(temp_dir.path(), false).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "visible.txt");
    }

    #[test]
    fn test_read_directory_hidden_files_shown() {
        let temp_dir = TempDir::new().unwrap();
        File::create(temp_dir.path().join("visible.txt")).unwrap();
        File::create(temp_dir.path().join(".hidden")).unwrap();

        let entries = FileEntry::read_directory(temp_dir.path(), true).unwrap();

        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_read_directory_sorting_dirs_first() {
        let temp_dir = TempDir::new().unwrap();
        File::create(temp_dir.path().join("aaa_file.txt")).unwrap();
        fs::create_dir(temp_dir.path().join("zzz_dir")).unwrap();

        let entries = FileEntry::read_directory(temp_dir.path(), false).unwrap();

        // Directory should come first even though it's alphabetically later
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "zzz_dir");
        assert_eq!(entries[0].file_type, FileType::Directory);
        assert_eq!(entries[1].name, "aaa_file.txt");
    }

    #[test]
    fn test_read_directory_sorting_alphabetical() {
        let temp_dir = TempDir::new().unwrap();
        File::create(temp_dir.path().join("charlie.txt")).unwrap();
        File::create(temp_dir.path().join("alpha.txt")).unwrap();
        File::create(temp_dir.path().join("bravo.txt")).unwrap();

        let entries = FileEntry::read_directory(temp_dir.path(), false).unwrap();

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "alpha.txt");
        assert_eq!(entries[1].name, "bravo.txt");
        assert_eq!(entries[2].name, "charlie.txt");
    }

    // ============================================
    // FileEntry::extension Tests
    // ============================================

    #[test]
    fn test_extension_single() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        File::create(&file_path).unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();
        assert_eq!(entry.extension(), Some("rs".to_string()));
    }

    #[test]
    fn test_extension_double() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.tar.gz");
        File::create(&file_path).unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();
        // Only gets last extension
        assert_eq!(entry.extension(), Some("gz".to_string()));
    }

    #[test]
    fn test_extension_none() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("README");
        File::create(&file_path).unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();
        assert_eq!(entry.extension(), None);
    }

    #[test]
    fn test_extension_dotfile() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join(".gitignore");
        File::create(&file_path).unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();
        // .gitignore is not treated as extension in std::path
        assert_eq!(entry.extension(), None);
    }

    #[test]
    fn test_extension_uppercase() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("photo.JPG");
        File::create(&file_path).unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();
        // Should be lowercase
        assert_eq!(entry.extension(), Some("jpg".to_string()));
    }

    // ============================================
    // FileEntry::format_size Tests
    // ============================================

    #[test]
    fn test_format_size_bytes() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("small.txt");
        fs::write(&file_path, "hello").unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();
        assert_eq!(entry.format_size(), "5 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("medium.txt");
        // Create a ~2KB file
        let content = "x".repeat(2048);
        fs::write(&file_path, content).unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();
        assert!(entry.format_size().contains("KB"));
    }

    // ============================================
    // FileEntry::format_modified Tests
    // ============================================

    #[test]
    fn test_format_modified_valid() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();
        let formatted = entry.format_modified();

        // Should be in YYYY-MM-DD HH:MM:SS format
        assert!(formatted.contains("-"));
        assert!(formatted.contains(":"));
        assert_eq!(formatted.len(), 19); // "2024-01-15 10:30:45"
    }

    // ============================================
    // FileEntry Clone Tests
    // ============================================

    #[test]
    fn test_file_entry_clone() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let entry = FileEntry::from_path(&file_path).unwrap();
        let cloned = entry.clone();

        assert_eq!(entry.name, cloned.name);
        assert_eq!(entry.path, cloned.path);
        assert_eq!(entry.file_type, cloned.file_type);
    }
}