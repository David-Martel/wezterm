use crate::file_entry::{FileEntry, FileType};

pub struct Icons;

impl Icons {
    pub fn get_icon(entry: &FileEntry) -> &'static str {
        match entry.file_type {
            FileType::Directory => "",
            FileType::Symlink => "",
            FileType::File => Self::get_file_icon(entry),
        }
    }

    fn get_file_icon(entry: &FileEntry) -> &'static str {
        if let Some(ext) = entry.extension() {
            match ext.as_str() {
                // Programming languages
                "rs" => "",
                "py" => "",
                "js" => "",
                "ts" => "",
                "jsx" | "tsx" => "",
                "go" => "",
                "java" => "",
                "c" | "h" => "",
                "cpp" | "cc" | "cxx" | "hpp" => "",
                "cs" => "",
                "php" => "",
                "rb" => "",
                "swift" => "",
                "kt" => "",
                "lua" => "",
                "vim" => "",
                "sh" | "bash" | "zsh" => "",
                "fish" => "",
                "ps1" | "psm1" => "",

                // Web
                "html" | "htm" => "",
                "css" | "scss" | "sass" | "less" => "",
                "json" => "",
                "xml" => "",
                "yaml" | "yml" => "",
                "toml" => "",
                "md" | "markdown" => "",

                // Documents
                "pdf" => "",
                "doc" | "docx" => "",
                "xls" | "xlsx" => "",
                "ppt" | "pptx" => "",
                "txt" => "",

                // Images
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "ico" | "webp" => "",

                // Videos
                "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => "",

                // Audio
                "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => "",

                // Archives
                "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" => "",

                // Databases
                "db" | "sqlite" | "sql" => "",

                // Git
                "git" => "",
                "gitignore" | "gitattributes" | "gitmodules" => "",

                // Docker
                "dockerfile" => "",

                // Config files
                "conf" | "config" | "ini" | "env" => "",

                // Lock files
                "lock" => "",

                // Logs
                "log" => "",

                _ => "",
            }
        } else {
            // Special files without extensions
            match entry.name.to_lowercase().as_str() {
                "readme" | "readme.md" => "",
                "license" | "license.md" => "",
                "makefile" => "",
                "dockerfile" => "",
                "cargo.toml" => "",
                "package.json" => "",
                ".gitignore" => "",
                ".dockerignore" => "",
                ".env" => "",
                _ => "",
            }
        }
    }

    pub fn get_color(entry: &FileEntry) -> ratatui::style::Color {
        use ratatui::style::Color;

        match entry.file_type {
            FileType::Directory => Color::Blue,
            FileType::Symlink => Color::Cyan,
            FileType::File => {
                if let Some(ext) = entry.extension() {
                    match ext.as_str() {
                        "rs" | "go" | "c" | "cpp" | "java" | "py" | "js" | "ts" => Color::Yellow,
                        "sh" | "bash" | "zsh" | "fish" | "ps1" => Color::Green,
                        "md" | "txt" | "pdf" | "doc" | "docx" => Color::White,
                        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" => Color::Magenta,
                        "mp4" | "mkv" | "avi" | "mov" => Color::Magenta,
                        "mp3" | "wav" | "flac" => Color::Magenta,
                        "zip" | "tar" | "gz" | "7z" | "rar" => Color::Red,
                        _ => Color::White,
                    }
                } else {
                    Color::White
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    // Helper to create a FileEntry for testing
    fn create_file_entry(name: &str, file_type: FileType) -> FileEntry {
        FileEntry {
            path: std::path::PathBuf::from(name),
            name: name.to_string(),
            file_type,
            size: 0,
            modified: SystemTime::now(),
            permissions: "rw-".to_string(),
            is_hidden: name.starts_with('.'),
        }
    }

    // ============================================
    // Directory Icon Tests
    // ============================================

    #[test]
    fn test_icon_directory() {
        let entry = create_file_entry("src", FileType::Directory);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, ""); // Directory icon
    }

    #[test]
    fn test_icon_symlink() {
        let entry = create_file_entry("link", FileType::Symlink);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, ""); // Symlink icon
    }

    // ============================================
    // Programming Language Icon Tests
    // ============================================

    #[test]
    fn test_icon_rust_file() {
        let entry = create_file_entry("main.rs", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_python_file() {
        let entry = create_file_entry("script.py", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_javascript_file() {
        let entry = create_file_entry("app.js", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_typescript_file() {
        let entry = create_file_entry("app.ts", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_go_file() {
        let entry = create_file_entry("main.go", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_c_file() {
        let entry = create_file_entry("main.c", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_cpp_file() {
        let entry = create_file_entry("main.cpp", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_shell_script() {
        let entry = create_file_entry("script.sh", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    // ============================================
    // Web File Icon Tests
    // ============================================

    #[test]
    fn test_icon_html_file() {
        let entry = create_file_entry("index.html", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_css_file() {
        let entry = create_file_entry("styles.css", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_json_file() {
        let entry = create_file_entry("config.json", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    // ============================================
    // Document Icon Tests
    // ============================================

    #[test]
    fn test_icon_markdown() {
        let entry = create_file_entry("README.md", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_pdf() {
        let entry = create_file_entry("document.pdf", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_text_file() {
        let entry = create_file_entry("notes.txt", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    // ============================================
    // Media Icon Tests
    // ============================================

    #[test]
    fn test_icon_image_file() {
        let entry = create_file_entry("photo.jpg", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_video_file() {
        let entry = create_file_entry("video.mp4", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_audio_file() {
        let entry = create_file_entry("music.mp3", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    // ============================================
    // Archive Icon Tests
    // ============================================

    #[test]
    fn test_icon_archive_zip() {
        let entry = create_file_entry("archive.zip", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_archive_tar() {
        let entry = create_file_entry("archive.tar", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    // ============================================
    // Config File Icon Tests
    // ============================================

    #[test]
    fn test_icon_config_file() {
        let entry = create_file_entry("settings.conf", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_toml_file() {
        let entry = create_file_entry("Cargo.toml", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_yaml_file() {
        let entry = create_file_entry("config.yaml", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    // ============================================
    // Special File Icon Tests
    // ============================================

    #[test]
    fn test_icon_gitignore() {
        let entry = create_file_entry(".gitignore", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_dockerfile() {
        let entry = create_file_entry("Dockerfile", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    #[test]
    fn test_icon_lock_file() {
        let entry = create_file_entry("Cargo.lock", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, "");
    }

    // ============================================
    // Unknown Extension Icon Tests
    // ============================================

    #[test]
    fn test_icon_unknown_extension() {
        let entry = create_file_entry("file.xyz", FileType::File);
        let icon = Icons::get_icon(&entry);
        assert_eq!(icon, ""); // Default file icon
    }

    // ============================================
    // Color Tests
    // ============================================

    #[test]
    fn test_color_directory() {
        use ratatui::style::Color;
        let entry = create_file_entry("src", FileType::Directory);
        let color = Icons::get_color(&entry);
        assert_eq!(color, Color::Blue);
    }

    #[test]
    fn test_color_symlink() {
        use ratatui::style::Color;
        let entry = create_file_entry("link", FileType::Symlink);
        let color = Icons::get_color(&entry);
        assert_eq!(color, Color::Cyan);
    }

    #[test]
    fn test_color_code_file() {
        use ratatui::style::Color;
        let entry = create_file_entry("main.rs", FileType::File);
        let color = Icons::get_color(&entry);
        assert_eq!(color, Color::Yellow);
    }

    #[test]
    fn test_color_script_file() {
        use ratatui::style::Color;
        let entry = create_file_entry("script.sh", FileType::File);
        let color = Icons::get_color(&entry);
        assert_eq!(color, Color::Green);
    }

    #[test]
    fn test_color_archive_file() {
        use ratatui::style::Color;
        let entry = create_file_entry("archive.zip", FileType::File);
        let color = Icons::get_color(&entry);
        assert_eq!(color, Color::Red);
    }

    #[test]
    fn test_color_media_file() {
        use ratatui::style::Color;
        let entry = create_file_entry("photo.png", FileType::File);
        let color = Icons::get_color(&entry);
        assert_eq!(color, Color::Magenta);
    }
}
