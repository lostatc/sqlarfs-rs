use std::fmt;
use std::io;
use std::result;

use thiserror::Error as DeriveError;

/// An opaque type that represents a SQLite error.
///
/// This type implements [`Debug`] and [`Display`], but not [`std::error::Error`]. Rather than try
/// to use this as an error type, you should use [`sqlarfs::Error::Sqlite`].
///
/// [`Debug`]: fmt::Debug
/// [`Display`]: fmt::Display
/// [`sqlarfs::Error::Sqlite`]: crate::Error::Sqlite
#[derive(Debug)]
pub struct SqliteError {
    inner: rusqlite::Error,
}

impl fmt::Display for SqliteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// The error type for sqlarfs.
///
/// This type can be converted [`From`] an [`std::io::Error`]. If the value the [`std::io::Error`]
/// wraps can be downcast into a [`sqlarfs::Error`], it will be. Otherwise, it will be converted
/// into [`sqlarfs::Error::Io`].
///
/// [`sqlarfs::Error`]: crate::Error
/// [`sqlarfs::Error::Io`]: crate::Error::Io
#[derive(Debug, DeriveError)]
#[non_exhaustive]
pub enum Error {
    /// A resource already exists.
    #[error("A resource already exists.")]
    AlreadyExists,

    /// A resource was not found.
    #[error("A resource was not found.")]
    NotFound,

    /// Some arguments were invalid.
    #[error("Some arguments were invalid.")]
    InvalidArgs,

    /// Attempted to read a compressed file, but the `deflate` Cargo feature was disabled.
    #[error("Attempted to read a compressed file, but the `deflate` Cargo feature was disabled.")]
    CompressionNotSupported,

    /// A file was modified in the database while you were trying to read or write to it.
    ///
    /// In SQLite parlance, this is called an ["expired
    /// blob"](https://sqlite.org/c3ref/blob_open.html).
    #[error(
        "This file was modified in the database while you were trying to read or write to it."
    )]
    BlobExpired,

    /// Attempted to write to a read-only archive or read-only file.
    #[error("Attempted to write to a read-only archive or read-only file.")]
    ReadOnly,

    /// There was an error with the underlying SQLite database.
    #[error("There was an error with the underlying SQLite database.\n{0}")]
    Sqlite(SqliteError),

    /// An I/O error occurred.
    #[error("{0}")]
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        let kind = error.kind();
        match error.into_inner() {
            Some(payload) => match payload.downcast::<Error>() {
                Ok(crate_error) => *crate_error,
                Err(other_error) => Error::Io(io::Error::new(kind, other_error)),
            },
            None => Error::Io(io::Error::from(kind)),
        }
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        // Don't use a default match arm here. We want to be explicit about how we're mapping
        // `Error` variants to `io::ErrorKind` variants and make sure we remember to update this
        // when we add new ones.
        let kind = match err {
            Error::AlreadyExists => io::ErrorKind::AlreadyExists,
            Error::NotFound => io::ErrorKind::NotFound,
            Error::InvalidArgs => io::ErrorKind::InvalidInput,
            Error::CompressionNotSupported => io::ErrorKind::InvalidInput,
            Error::BlobExpired => io::ErrorKind::Other,
            Error::ReadOnly => io::ErrorKind::Other,
            Error::Sqlite(_) => io::ErrorKind::Other,
            Error::Io(err) => return err,
        };

        io::Error::new(kind, err)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        match err.sqlite_error_code() {
            Some(rusqlite::ErrorCode::ReadOnly) => Self::ReadOnly,
            _ => Self::Sqlite(SqliteError { inner: err }),
        }
    }
}

pub fn io_err_has_sqlite_code(err: &io::Error, code: rusqlite::ErrorCode) -> bool {
    if let Some(payload) = err.get_ref() {
        if let Some(sqlite_err) = payload.downcast_ref::<rusqlite::Error>() {
            return sqlite_err.sqlite_error_code() == Some(code);
        }
    }

    false
}

/// The result type for operations with a repository.
pub type Result<T> = result::Result<T, Error>;
