use crate::sftp::types::Metadata;
use crate::sftp::SftpChannelResult;

#[cfg(feature = "russh")]
use crate::russh_backend::{block_on, RusshFile};

pub(crate) enum FileWrap {
    #[cfg(feature = "ssh2")]
    Ssh2(ssh2::File),

    #[cfg(feature = "libssh-rs")]
    LibSsh(libssh_rs::SftpFile),

    #[cfg(feature = "russh")]
    Russh(RusshFile),
}

/// Synchronous reader wrapper for russh async file.
#[cfg(feature = "russh")]
struct RusshFileReader<'a> {
    file: &'a RusshFile,
}

#[cfg(feature = "russh")]
impl<'a> std::io::Read for RusshFileReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        block_on(self.file.read(buf)).map_err(|e| std::io::Error::other(e.to_string()))
    }
}

/// Synchronous writer wrapper for russh async file.
#[cfg(feature = "russh")]
struct RusshFileWriter<'a> {
    file: &'a RusshFile,
}

#[cfg(feature = "russh")]
impl<'a> std::io::Write for RusshFileWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        block_on(self.file.write(buf)).map_err(|e| std::io::Error::other(e.to_string()))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        block_on(self.file.flush()).map_err(|e| std::io::Error::other(e.to_string()))
    }
}

impl FileWrap {
    pub fn reader(&mut self) -> Box<dyn std::io::Read + '_> {
        match self {
            #[cfg(feature = "ssh2")]
            Self::Ssh2(file) => Box::new(file),

            #[cfg(feature = "libssh-rs")]
            Self::LibSsh(file) => Box::new(file),

            #[cfg(feature = "russh")]
            Self::Russh(file) => Box::new(RusshFileReader { file }),
        }
    }

    pub fn writer(&mut self) -> Box<dyn std::io::Write + '_> {
        match self {
            #[cfg(feature = "ssh2")]
            Self::Ssh2(file) => Box::new(file),

            #[cfg(feature = "libssh-rs")]
            Self::LibSsh(file) => Box::new(file),

            #[cfg(feature = "russh")]
            Self::Russh(file) => Box::new(RusshFileWriter { file }),
        }
    }

    pub fn set_metadata(
        &mut self,
        #[cfg_attr(not(any(feature = "ssh2", feature = "russh")), allow(unused_variables))]
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
            Self::Russh(file) => block_on(file.set_metadata(metadata)),
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
            Self::Russh(file) => block_on(file.metadata()),
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
            Self::Russh(file) => block_on(file.flush()),
        }
    }
}
