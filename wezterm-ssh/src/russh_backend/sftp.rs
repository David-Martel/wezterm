//! SFTP implementation using russh-sftp.
//!
//! This module provides [`RusshSftp`], [`RusshFile`], and [`RusshDir`] types
//! for secure file transfer operations over SSH.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     wezterm-ssh (sync)                       │
//! │                           │                                  │
//! │                      block_on()                              │
//! │                           ▼                                  │
//! │  ┌──────────────────────────────────────────────────────┐   │
//! │  │                   RusshSftp                           │   │
//! │  │  ┌────────────┐  ┌────────────┐  ┌────────────┐      │   │
//! │  │  │ RusshFile  │  │ RusshDir   │  │  metadata  │      │   │
//! │  │  │ read/write │  │  iteration │  │  symlinks  │      │   │
//! │  │  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘      │   │
//! │  │        └───────────────┴───────────────┘              │   │
//! │  │                        │                              │   │
//! │  │                 SftpSession                           │   │
//! │  │                   (async)                             │   │
//! │  └──────────────────────────────────────────────────────┘   │
//! │                           │                                  │
//! │                    SSH Channel                               │
//! │                           │                                  │
//! │                    ───────┴───────                           │
//! │                      Network I/O                             │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Supported Operations
//!
//! | Category | Operations |
//! |----------|------------|
//! | **Files** | open, read, write, seek, flush, close |
//! | **Directories** | create_dir, remove_dir, read_dir, open_dir |
//! | **Metadata** | metadata, symlink_metadata, set_metadata |
//! | **Links** | symlink, read_link, canonicalize |
//! | **Management** | unlink, rename |
//!
//! ## Type Conversions
//!
//! This module handles conversions between wezterm-ssh and russh-sftp types:
//!
//! | wezterm-ssh | russh-sftp |
//! |-------------|------------|
//! | `Metadata` | `FileAttributes` |
//! | `OpenOptions` | `OpenFlags` |
//! | `FileType` | Unix mode bits |
//! | `FilePermissions` | Unix permission bits |
//!
//! ## Thread Safety
//!
//! All types use `Arc<Mutex<T>>` internally and are safe to share across
//! threads. The underlying SFTP session is accessed through async locks
//! to prevent concurrent modification.

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use russh::Channel;
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::{FileAttributes, OpenFlags};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::sftp::types::{Metadata, OpenOptions, RenameOptions};
use crate::sftp::{SftpChannelError, SftpChannelResult};

/// SFTP session wrapper for russh.
///
/// Provides file transfer operations over an SSH channel using the SFTP
/// protocol (SSH File Transfer Protocol). All operations are async internally
/// but exposed via `block_on()` for wezterm-ssh's sync interface.
///
/// ## Example
///
/// ```ignore
/// // Create SFTP session from SSH channel
/// let sftp = RusshSftp::new(channel).await?;
///
/// // Read directory
/// let entries = sftp.read_dir("/home/user".into()).await?;
///
/// // Open file for reading
/// let opts = OpenOptions { read: true, ..Default::default() };
/// let file = sftp.open("/home/user/file.txt".into(), opts).await?;
/// ```
pub struct RusshSftp {
    session: Arc<Mutex<SftpSession>>,
}

impl RusshSftp {
    /// Create a new SFTP session from an SSH channel.
    pub async fn new(channel: Channel<russh::client::Msg>) -> anyhow::Result<Self> {
        let sftp = SftpSession::new(channel.into_stream())
            .await
            .context("Failed to initialize SFTP session")?;

        Ok(Self {
            session: Arc::new(Mutex::new(sftp)),
        })
    }

    /// Open a file with the specified options.
    pub async fn open(&self, path: &Utf8Path, opts: OpenOptions) -> SftpChannelResult<RusshFile> {
        let session = self.session.lock().await;

        // Convert OpenOptions to russh-sftp OpenFlags
        let flags = convert_open_options(&opts);

        // Set file mode (permissions)
        let attrs = FileAttributes {
            permissions: Some(opts.mode as u32),
            ..Default::default()
        };

        let file = session
            .open_with_flags_and_attributes(path.as_str(), flags, attrs)
            .await
            .map_err(|e| {
                SftpChannelError::from(std::io::Error::other(format!("SFTP open error: {}", e)))
            })?;

        Ok(RusshFile {
            inner: Arc::new(Mutex::new(Some(file))),
        })
    }

    /// Create a symbolic link.
    pub async fn symlink(&self, path: &Utf8Path, target: &Utf8Path) -> SftpChannelResult<()> {
        let session = self.session.lock().await;
        session
            .symlink(path.as_str(), target.as_str())
            .await
            .map_err(|e| {
                SftpChannelError::from(std::io::Error::other(format!("SFTP symlink error: {}", e)))
            })
    }

    /// Read the target of a symbolic link.
    pub async fn read_link(&self, path: &Utf8Path) -> SftpChannelResult<Utf8PathBuf> {
        let session = self.session.lock().await;
        let target = session.read_link(path.as_str()).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!(
                "SFTP read_link error: {}",
                e
            )))
        })?;

        Ok(Utf8PathBuf::from(target))
    }

    /// Canonicalize a path (resolve to absolute path).
    pub async fn canonicalize(&self, path: &Utf8Path) -> SftpChannelResult<Utf8PathBuf> {
        let session = self.session.lock().await;
        let canonical = session.canonicalize(path.as_str()).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!(
                "SFTP canonicalize error: {}",
                e
            )))
        })?;

        Ok(Utf8PathBuf::from(canonical))
    }

    /// Remove a file.
    pub async fn unlink(&self, path: &Utf8Path) -> SftpChannelResult<()> {
        let session = self.session.lock().await;
        session.remove_file(path.as_str()).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!("SFTP unlink error: {}", e)))
        })
    }

    /// Remove a directory.
    pub async fn remove_dir(&self, path: &Utf8Path) -> SftpChannelResult<()> {
        let session = self.session.lock().await;
        session.remove_dir(path.as_str()).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!(
                "SFTP remove_dir error: {}",
                e
            )))
        })
    }

    /// Create a directory.
    pub async fn create_dir(&self, path: &Utf8Path, mode: i32) -> SftpChannelResult<()> {
        let session = self.session.lock().await;

        // Create with specified permissions
        let attrs = FileAttributes {
            permissions: Some(mode as u32),
            ..Default::default()
        };

        session.create_dir(path.as_str()).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!(
                "SFTP create_dir error: {}",
                e
            )))
        })?;

        // Try to set permissions (some servers may not support this)
        let _ = session.set_metadata(path.as_str(), attrs).await;

        Ok(())
    }

    /// Rename a file or directory.
    pub async fn rename(
        &self,
        src: &Utf8Path,
        dest: &Utf8Path,
        _opts: RenameOptions,
    ) -> SftpChannelResult<()> {
        let session = self.session.lock().await;
        session
            .rename(src.as_str(), dest.as_str())
            .await
            .map_err(|e| {
                SftpChannelError::from(std::io::Error::other(format!("SFTP rename error: {}", e)))
            })
    }

    /// Get metadata for a path (follows symlinks).
    pub async fn metadata(&self, path: &Utf8Path) -> SftpChannelResult<Metadata> {
        let session = self.session.lock().await;
        let attrs = session.metadata(path.as_str()).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!("SFTP metadata error: {}", e)))
        })?;

        Ok(convert_file_attributes(attrs))
    }

    /// Get metadata for a symlink (does not follow).
    pub async fn symlink_metadata(&self, path: &Utf8Path) -> SftpChannelResult<Metadata> {
        let session = self.session.lock().await;
        let attrs = session.symlink_metadata(path.as_str()).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!(
                "SFTP symlink_metadata error: {}",
                e
            )))
        })?;

        Ok(convert_file_attributes(attrs))
    }

    /// Set metadata for a path.
    pub async fn set_metadata(&self, path: &Utf8Path, metadata: Metadata) -> SftpChannelResult<()> {
        let session = self.session.lock().await;
        let attrs = convert_metadata_to_attrs(metadata);

        session
            .set_metadata(path.as_str(), attrs)
            .await
            .map_err(|e| {
                SftpChannelError::from(std::io::Error::other(format!(
                    "SFTP set_metadata error: {}",
                    e
                )))
            })
    }

    /// Read directory contents.
    pub async fn read_dir(
        &self,
        path: &Utf8Path,
    ) -> SftpChannelResult<Vec<(Utf8PathBuf, Metadata)>> {
        let session = self.session.lock().await;
        let read_dir = session.read_dir(path.as_str()).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!("SFTP read_dir error: {}", e)))
        })?;

        let mut entries = Vec::new();
        for entry in read_dir {
            let name = entry.file_name();
            // Skip . and ..
            if name == "." || name == ".." {
                continue;
            }
            let file_path = path.join(&name);
            let metadata = convert_file_attributes(entry.metadata());
            entries.push((file_path, metadata));
        }

        Ok(entries)
    }

    /// Open a directory for iteration.
    pub async fn open_dir(&self, path: &Utf8Path) -> SftpChannelResult<RusshDir> {
        let session = self.session.lock().await;
        let read_dir = session.read_dir(path.as_str()).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!("SFTP open_dir error: {}", e)))
        })?;

        // Collect all entries immediately since russh_sftp doesn't support
        // incremental iteration
        let entries: Vec<_> = read_dir
            .filter_map(|entry| {
                let name = entry.file_name();
                if name == "." || name == ".." {
                    None
                } else {
                    Some((path.join(&name), convert_file_attributes(entry.metadata())))
                }
            })
            .collect();

        Ok(RusshDir {
            entries: Arc::new(Mutex::new(entries.into_iter())),
        })
    }
}

/// SFTP file handle wrapper.
///
/// Provides async read/write/seek operations on a remote file.
/// The file is automatically closed when dropped if not explicitly closed.
///
/// ## Operations
///
/// | Method | Description |
/// |--------|-------------|
/// | [`read`](Self::read) | Read bytes from current position |
/// | [`write`](Self::write) | Write bytes at current position |
/// | [`seek`](Self::seek) | Change file position |
/// | [`flush`](Self::flush) | Ensure data is written to server |
/// | [`metadata`](Self::metadata) | Get file attributes |
/// | [`close`](Self::close) | Explicitly close file handle |
pub struct RusshFile {
    inner: Arc<Mutex<Option<russh_sftp::client::fs::File>>>,
}

impl RusshFile {
    /// Read from the file.
    pub async fn read(&self, buf: &mut [u8]) -> SftpChannelResult<usize> {
        let mut guard = self.inner.lock().await;
        let file = guard
            .as_mut()
            .ok_or_else(|| SftpChannelError::from(std::io::Error::other("File already closed")))?;

        file.read(buf).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!("SFTP read error: {}", e)))
        })
    }

    /// Write to the file.
    pub async fn write(&self, buf: &[u8]) -> SftpChannelResult<usize> {
        let mut guard = self.inner.lock().await;
        let file = guard
            .as_mut()
            .ok_or_else(|| SftpChannelError::from(std::io::Error::other("File already closed")))?;

        file.write(buf).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!("SFTP write error: {}", e)))
        })
    }

    /// Flush the file.
    pub async fn flush(&self) -> SftpChannelResult<()> {
        let mut guard = self.inner.lock().await;
        let file = guard
            .as_mut()
            .ok_or_else(|| SftpChannelError::from(std::io::Error::other("File already closed")))?;

        file.flush().await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!("SFTP flush error: {}", e)))
        })
    }

    /// Seek within the file.
    pub async fn seek(&self, pos: std::io::SeekFrom) -> SftpChannelResult<u64> {
        let mut guard = self.inner.lock().await;
        let file = guard
            .as_mut()
            .ok_or_else(|| SftpChannelError::from(std::io::Error::other("File already closed")))?;

        file.seek(pos).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!("SFTP seek error: {}", e)))
        })
    }

    /// Get file metadata.
    pub async fn metadata(&self) -> SftpChannelResult<Metadata> {
        let guard = self.inner.lock().await;
        let file = guard
            .as_ref()
            .ok_or_else(|| SftpChannelError::from(std::io::Error::other("File already closed")))?;

        let attrs = file.metadata().await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!("SFTP metadata error: {}", e)))
        })?;

        Ok(convert_file_attributes(attrs))
    }

    /// Set file metadata.
    pub async fn set_metadata(&self, metadata: Metadata) -> SftpChannelResult<()> {
        let guard = self.inner.lock().await;
        let file = guard
            .as_ref()
            .ok_or_else(|| SftpChannelError::from(std::io::Error::other("File already closed")))?;

        let attrs = convert_metadata_to_attrs(metadata);
        file.set_metadata(attrs).await.map_err(|e| {
            SftpChannelError::from(std::io::Error::other(format!(
                "SFTP set_metadata error: {}",
                e
            )))
        })
    }

    /// Close the file.
    pub async fn close(&self) -> SftpChannelResult<()> {
        let mut guard = self.inner.lock().await;
        // Take the file out, which will drop it
        let _ = guard.take();
        Ok(())
    }
}

/// SFTP directory iterator wrapper.
///
/// Provides iteration over directory entries. All entries are fetched
/// upfront since russh-sftp doesn't support incremental iteration.
///
/// Entries named `.` and `..` are automatically filtered out.
pub struct RusshDir {
    entries: Arc<Mutex<std::vec::IntoIter<(Utf8PathBuf, Metadata)>>>,
}

impl RusshDir {
    /// Get the next directory entry.
    pub async fn next(&self) -> Option<(Utf8PathBuf, Metadata)> {
        let mut guard = self.entries.lock().await;
        guard.next()
    }
}

/// Convert wezterm [`OpenOptions`] to russh-sftp [`OpenFlags`].
///
/// Maps the high-level open options to SFTP protocol flags:
/// - `read: true` → `OpenFlags::READ`
/// - `write: WriteMode::Write` → `OpenFlags::WRITE | CREATE | TRUNCATE`
/// - `write: WriteMode::Append` → `OpenFlags::WRITE | APPEND`
fn convert_open_options(opts: &OpenOptions) -> OpenFlags {
    use crate::sftp::types::{OpenFileType, WriteMode};

    let mut flags = OpenFlags::empty();

    // Read flag
    if opts.read {
        flags |= OpenFlags::READ;
    }

    // Write flags
    match opts.write {
        Some(WriteMode::Write) => {
            flags |= OpenFlags::WRITE;
            if opts.ty == OpenFileType::File {
                // For new files, also set create and truncate
                flags |= OpenFlags::CREATE | OpenFlags::TRUNCATE;
            }
        }
        Some(WriteMode::Append) => {
            flags |= OpenFlags::WRITE | OpenFlags::APPEND;
        }
        None => {}
    }

    // Create if it doesn't exist
    if opts.ty == OpenFileType::File && opts.write.is_some() {
        flags |= OpenFlags::CREATE;
    }

    flags
}

/// Convert russh-sftp [`FileAttributes`] to wezterm [`Metadata`].
///
/// Handles the mapping of:
/// - File type extraction from Unix mode bits
/// - Permission bits to `FilePermissions`
/// - Timestamp conversion (u32 → u64)
/// - Optional uid/gid preservation
fn convert_file_attributes(attrs: FileAttributes) -> Metadata {
    use crate::sftp::types::{FilePermissions, FileType};

    // Determine file type from permissions mode bits
    let ty = if let Some(perms) = attrs.permissions {
        FileType::from_unix_mode(perms)
    } else {
        FileType::File // Default to file if permissions not available
    };

    // Convert permissions
    let permissions = attrs.permissions.map(FilePermissions::from_unix_mode);

    Metadata {
        ty,
        size: attrs.size,
        uid: attrs.uid,
        gid: attrs.gid,
        permissions,
        accessed: attrs.atime.map(|t| t as u64),
        modified: attrs.mtime.map(|t| t as u64),
    }
}

/// Convert wezterm [`Metadata`] to russh-sftp [`FileAttributes`].
///
/// Reconstructs Unix mode by combining:
/// - File type bits from `metadata.ty.to_unix_mode()`
/// - Permission bits from `metadata.permissions.to_unix_mode()`
///
/// Timestamps are converted from u64 to u32 (safe for dates before 2038).
fn convert_metadata_to_attrs(metadata: Metadata) -> FileAttributes {
    // Combine file type bits with permission bits
    let permissions = metadata.permissions.map(|p| {
        let perm_bits = p.to_unix_mode();
        let type_bits = metadata.ty.to_unix_mode();
        perm_bits | type_bits
    });

    FileAttributes {
        size: metadata.size,
        uid: metadata.uid,
        gid: metadata.gid,
        permissions,
        atime: metadata.accessed.map(|t| t as u32),
        mtime: metadata.modified.map(|t| t as u32),
        user: None,
        group: None,
    }
}
