use crate::sftp::types::Metadata;
use crate::sftp::{SftpChannelError, SftpChannelResult};

pub(crate) enum FileWrap {
    #[cfg(feature = "ssh2")]
    Ssh2(ssh2::File),

    #[cfg(feature = "libssh-rs")]
    LibSsh(libssh_rs::SftpFile),

    #[cfg(feature = "russh")]
    Russh(RusshFilePlaceholder),
}

/// Placeholder for russh SFTP file implementation.
#[cfg(feature = "russh")]
pub(crate) struct RusshFilePlaceholder;

#[cfg(feature = "russh")]
fn russh_file_not_implemented<T>() -> SftpChannelResult<T> {
    Err(SftpChannelError::from(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "SFTP file operations not yet implemented for russh backend",
    )))
}

impl FileWrap {
    pub fn reader(&mut self) -> Box<dyn std::io::Read + '_> {
        match self {
            #[cfg(feature = "ssh2")]
            Self::Ssh2(file) => Box::new(file),

            #[cfg(feature = "libssh-rs")]
            Self::LibSsh(file) => Box::new(file),

            #[cfg(feature = "russh")]
            Self::Russh(_) => {
                panic!("russh SFTP file reader not implemented")
            }
        }
    }

    pub fn writer(&mut self) -> Box<dyn std::io::Write + '_> {
        match self {
            #[cfg(feature = "ssh2")]
            Self::Ssh2(file) => Box::new(file),

            #[cfg(feature = "libssh-rs")]
            Self::LibSsh(file) => Box::new(file),

            #[cfg(feature = "russh")]
            Self::Russh(_) => {
                panic!("russh SFTP file writer not implemented")
            }
        }
    }

    pub fn set_metadata(
        &mut self,
        #[cfg_attr(
            not(any(feature = "ssh2", feature = "russh")),
            allow(unused_variables)
        )]
        metadata: Metadata,
    ) -> SftpChannelResult<()> {
        match self {
            #[cfg(feature = "ssh2")]
            Self::Ssh2(file) => Ok(file.setstat(metadata.into())?),

            #[cfg(feature = "libssh-rs")]
            Self::LibSsh(_file) => Err(libssh_rs::Error::fatal(
                "FileWrap::set_metadata not implemented for libssh::SftpFile",
            )
            .into()),

            #[cfg(feature = "russh")]
            Self::Russh(_) => {
                let _ = metadata;
                russh_file_not_implemented()
            }
        }
    }

    pub fn metadata(&mut self) -> SftpChannelResult<Metadata> {
        match self {
            #[cfg(feature = "ssh2")]
            Self::Ssh2(file) => Ok(file.stat().map(Metadata::from)?),

            #[cfg(feature = "libssh-rs")]
            Self::LibSsh(file) => file
                .metadata()
                .map(Metadata::from)
                .map_err(SftpChannelError::from),

            #[cfg(feature = "russh")]
            Self::Russh(_) => russh_file_not_implemented(),
        }
    }

    pub fn fsync(&mut self) -> SftpChannelResult<()> {
        match self {
            #[cfg(feature = "ssh2")]
            Self::Ssh2(file) => file.fsync().map_err(SftpChannelError::from),

            #[cfg(feature = "libssh-rs")]
            Self::LibSsh(file) => {
                use std::io::Write;
                Ok(file.flush()?)
            }

            #[cfg(feature = "russh")]
            Self::Russh(_) => russh_file_not_implemented(),
        }
    }
}
