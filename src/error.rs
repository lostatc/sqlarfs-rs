use std::fmt;
use std::io;
use std::result;

use thiserror::Error as DeriveError;

#[derive(Debug)]
pub struct SqlError {
    inner: rusqlite::Error,
}

impl fmt::Display for SqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// The error type for sqlarfs.
#[derive(Debug, DeriveError)]
#[non_exhaustive]
pub enum Error {
    /// A resource already exists.
    #[error("A resource already exists.")]
    AlreadyExists,

    /// A resource was not found.
    #[error("A resource was not found.")]
    NotFound,

    /// A file is not seekable because it's compressed.
    #[error("This file is compressed and therefore not seekable.")]
    NotSeekable,

    /// There was an error with the underlying SQLite database.
    #[error("There was an error with the underlying SQLite database.\n{0}")]
    Sql(SqlError),

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
            // When it's stable, we can use `io::ErrorKind::NotSeekable`.
            // https://doc.rust-lang.org/std/io/enum.ErrorKind.html#variant.NotSeekable
            Error::NotSeekable => io::ErrorKind::Other,
            Error::Sql(_) => io::ErrorKind::Other,
            Error::Io(err) => return err,
        };

        io::Error::new(kind, err)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sql(SqlError { inner: err })
    }
}

/// The result type for operations with a repository.
pub type Result<T> = result::Result<T, Error>;
