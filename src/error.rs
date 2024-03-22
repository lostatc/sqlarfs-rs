use std::io;
use std::result;

use thiserror::Error as DeriveError;

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
            Error::Io(err) => return err,
        };

        io::Error::new(kind, err)
    }
}

/// The result type for operations with a repository.
pub type Result<T> = result::Result<T, Error>;
