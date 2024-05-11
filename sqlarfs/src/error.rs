use std::fmt;
use std::io;
use std::path::PathBuf;
use std::result;
use thiserror::Error;

/// An opaque type representing a SQLite error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqliteErrorCode {
    extended_code: std::ffi::c_int,
}

impl fmt::Display for SqliteErrorCode {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        rusqlite::ffi::Error::new(self.extended_code).fmt(f)
    }
}

impl SqliteErrorCode {
    /// The raw extended error code from the SQLite C API.
    ///
    /// See the [SQLite docs](https://www.sqlite.org/rescode.html) for more information.
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn raw_code(&self) -> std::ffi::c_int {
        // We're not including rusqlite in our public API, so we're only exposing the raw error
        // code from the SQLite C API as opposed to any rusqlite types.
        self.extended_code
    }
}

/// The error type for sqlarfs.
///
/// This type can be converted [`From`] an [`std::io::Error`]. If the value the [`std::io::Error`]
/// wraps can be downcast into a [`sqlarfs::Error`], it will be. Otherwise, it will be converted
/// into a new [`sqlarfs::Error::Io`].
///
/// [`sqlarfs::Error`]: crate::Error
/// [`sqlarfs::ErrorKind::Io`]: crate::ErrorKind::Io
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    // As a rule, we don't document this error kind as a possible error return in the API docs
    // because a) there may be several possible cases where it could be returned and b) it
    // generally represents an error on the part of the user and isn't useful to catch. We may
    // still document the circumstances that could lean to this error, but not that this specific
    // error kind would be returned.
    #[error("Some arguments were invalid: {reason}")]
    InvalidArgs { reason: String },

    #[error("This file already exists: {path}")]
    FileAlreadyExists { path: PathBuf },

    #[error("This file was not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("This file has no parent directory: {path}")]
    NoParentDirectory { path: PathBuf },

    #[error("This file is a directory or a symbolic link, when we were expecting a regular file: {path}")]
    NotARegularFile { path: PathBuf },

    #[error("A file is not a directory, when we were expecting one: {path}")]
    NotADirectory { path: PathBuf },

    #[error("A loop of symbolic links was encountered while traversing the filesystem.")]
    FilesystemLoop,

    #[error("Attempted to read a compressed file, but sqlarfs was compiled without compression support.")]
    CompressionNotSupported,

    #[error(
        "Attempted to write more data to the SQLite archive than its maximum blob size will allow."
    )]
    FileTooBig,

    #[error("Attempted to write to a read-only database.")]
    ReadOnly,

    #[error("There was an error from the underlying SQLite database: {code:?}")]
    Sqlite {
        /// The underlying SQLite error code, if there is one.
        code: Option<SqliteErrorCode>,
    },

    #[error("An I/O error occurred: {kind}")]
    Io {
        /// The [`std::io::ErrorKind`] of the I/O error.
        kind: io::ErrorKind,
    },
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        let kind = error.kind();
        match error.into_inner() {
            Some(payload) => match payload.downcast::<Error>() {
                Ok(crate_error) => *crate_error,
                Err(other_error) => Error::Io { kind },
            },
            None => Error::Io { kind },
        }
    }
}

impl From<Error> for io::Error {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn from(err: Error) -> Self {
        // Don't use a default match arm here. We want to be explicit about how we're mapping
        // `ErrorKind` variants to `io::ErrorKind` variants and make sure we remember to update
        // this when we add new ones.
        let kind = match err {
            Error::InvalidArgs { .. } => io::ErrorKind::InvalidInput,
            Error::FileAlreadyExists { .. } => io::ErrorKind::AlreadyExists,
            Error::FileNotFound { .. } => io::ErrorKind::NotFound,
            Error::NoParentDirectory { .. } => io::ErrorKind::NotFound,
            Error::NotARegularFile { .. } => io::ErrorKind::Other,
            // When it's stable, we can use `std::io::ErrorKind::NotADirectory`.
            Error::NotADirectory { .. } => io::ErrorKind::Other,
            // When it's stable, we can use `std::io::ErrorKind::FilesystemLoop`.
            Error::FilesystemLoop => io::ErrorKind::Other,
            Error::CompressionNotSupported => io::ErrorKind::Other,
            Error::FileTooBig => io::ErrorKind::Other,
            Error::ReadOnly => io::ErrorKind::Other,
            Error::Sqlite { .. } => io::ErrorKind::Other,
            Error::Io { kind } => kind,
        };

        io::Error::new(kind, err)
    }
}

impl From<rusqlite::Error> for Error {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn from(err: rusqlite::Error) -> Self {
        match err.sqlite_error() {
            Some(rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::ReadOnly,
                ..
            }) => Error::ReadOnly,
            Some(rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::TooBig,
                ..
            }) => Error::FileTooBig,
            code => Error::Sqlite {
                code: code.map(|code| SqliteErrorCode {
                    extended_code: code.extended_code,
                }),
            },
        }
    }
}

/// The result type for sqlarfs.
pub type Result<T> = result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use xpct::{be_ok, be_some, equal, expect, match_pattern, pattern};

    use super::*;

    #[test]
    fn convert_sqlarfs_io_err_into_std_io_error() {
        let err = Error::Io {
            kind: io::ErrorKind::NotFound,
        };

        let io_err: io::Error = err.into();

        expect!(io_err.kind()).to(equal(io::ErrorKind::NotFound));

        expect!(io_err.into_inner())
            .to(be_some())
            .map(|err| err.downcast::<Error>())
            .to(be_ok())
            .to(equal(Box::new(Error::Io {
                kind: io::ErrorKind::NotFound,
            })));
    }

    #[test]
    fn convert_into_io_error_with_kind() {
        let err: Error = Error::FileNotFound {
            path: PathBuf::new(),
        }
        .into();

        let io_err: io::Error = err.into();

        expect!(io_err.kind()).to(equal(io::ErrorKind::NotFound));
    }

    #[test]
    fn convert_from_io_error_with_kind() {
        let io_err: io::Error = io::ErrorKind::NotFound.into();
        let err: Error = io_err.into();

        expect!(err).to(equal(Error::Io {
            kind: io::ErrorKind::NotFound,
        }));
    }

    #[test]
    fn convert_from_io_error_wrapping_a_sqlarfs_error() {
        let original_err: Error = Error::InvalidArgs {
            reason: String::new(),
        }
        .into();
        let io_err: io::Error = original_err.into();
        let unwrapped_error: Error = io_err.into();

        expect!(unwrapped_error).to(match_pattern(pattern!(Error::InvalidArgs { .. })));
    }

    #[test]
    fn convert_from_rusqlite_error() {
        let rusqlite_err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::ReadOnly,
                extended_code: 0,
            },
            None,
        );

        let err: Error = rusqlite_err.into();

        expect!(err).to(equal(Error::ReadOnly));
    }
}
