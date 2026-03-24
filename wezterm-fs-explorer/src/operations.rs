use anyhow::Result;
use std::fs;
use std::path::Path;

pub struct FileOperation;

impl FileOperation {
    pub fn delete(path: &Path) -> Result<()> {
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn rename(old_path: &Path, new_path: &Path) -> Result<()> {
        fs::rename(old_path, new_path)?;
        Ok(())
    }

    pub fn copy(source: &Path, dest: &Path) -> Result<()> {
        if source.is_dir() {
            Self::copy_dir_all(source, dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source, dest)?;
        }
        Ok(())
    }

    pub fn create_file(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::File::create(path)?;
        Ok(())
    }

    pub fn create_directory(path: &Path) -> Result<()> {
        fs::create_dir_all(path)?;
        Ok(())
    }

    fn copy_dir_all(source: &Path, dest: &Path) -> Result<()> {
        fs::create_dir_all(dest)?;

        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let entry_path = entry.path();
            let file_name = entry.file_name();
            let dest_path = dest.join(file_name);

            if entry_path.is_dir() {
                Self::copy_dir_all(&entry_path, &dest_path)?;
            } else {
                fs::copy(&entry_path, &dest_path)?;
            }
        }

        Ok(())
    }
}
