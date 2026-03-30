use thiserror::Error;

#[expect(dead_code, reason = "Scaffolded error type for future migration from anyhow::Result to typed errors; \
    call sites currently use anyhow for convenience but will switch to ExplorerError \
    when the crate is promoted to a workspace member with public API surface")]
#[derive(Error, Debug)]
pub enum ExplorerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Git error: {0}")]
    Git(String),
}
