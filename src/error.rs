use std::fmt;
use std::io;
use std::result;

/// The error type for sqlarfs.
///
/// This type can be converted [`From`] an [`std::io::Error`]. If the value the [`std::io::Error`]
/// wraps can be downcast into a [`sqlarfs::Error`], it will be. Otherwise, it will be converted
/// into a new [`sqlarfs::Error`] with the [`sqlarfs::ErrorKind::Io`] kind.
///
/// [`sqlarfs::Error`]: crate::Error
/// [`sqlarfs::ErrorKind::Io`]: crate::ErrorKind::Io
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    source: Option<anyhow::Error>,
}

impl Error {
    /// Create a new [`Error`] wrapping the given `source` error.
    pub fn new<E>(kind: ErrorKind, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            kind,
            source: Some(source.into()),
        }
    }

    /// The [`ErrorKind`] of this error.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Consume this error and return its [`ErrorKind`].
    pub fn into_kind(self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|err| err.as_ref())
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self { kind, source: None }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        let kind = error.kind();
        match error.into_inner() {
            Some(payload) => match payload.downcast::<Error>() {
                Ok(crate_error) => *crate_error,
                Err(other_error) => {
                    Error::new(ErrorKind::Io { kind }, io::Error::new(kind, other_error))
                }
            },
            None => Error::new(ErrorKind::Io { kind }, io::Error::from(kind)),
        }
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        // Don't use a default match arm here. We want to be explicit about how we're mapping
        // `ErrorKind` variants to `io::ErrorKind` variants and make sure we remember to update
        // this when we add new ones.
        let kind = match err.kind() {
            ErrorKind::AlreadyExists => io::ErrorKind::AlreadyExists,
            ErrorKind::NotFound => io::ErrorKind::NotFound,
            ErrorKind::InvalidArgs => io::ErrorKind::InvalidInput,
            ErrorKind::CompressionNotSupported => io::ErrorKind::InvalidInput,
            ErrorKind::BlobExpired => io::ErrorKind::Other,
            ErrorKind::FileTooBig => io::ErrorKind::Other,
            ErrorKind::ReadOnly => io::ErrorKind::Other,
            ErrorKind::Sqlite => io::ErrorKind::Other,
            ErrorKind::Io { kind } => *kind,
        };

        io::Error::new(kind, err)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        let kind = match err.sqlite_error_code() {
            Some(rusqlite::ErrorCode::ReadOnly) => ErrorKind::ReadOnly,
            Some(rusqlite::ErrorCode::TooBig) => ErrorKind::FileTooBig,
            _ => ErrorKind::Sqlite,
        };

        Self::new(kind, err)
    }
}

/// A category of [`Error`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    // If you update one of these doc comments, you may also want to update the
    // [`std::fmt::Display`] impl.
    /// A resource already exists.
    AlreadyExists,

    /// A resource was not found.
    NotFound,

    /// Some arguments were invalid.
    InvalidArgs,

    /// Attempted to read a compressed file, but the `deflate` Cargo feature was disabled.
    CompressionNotSupported,

    /// A file was modified in the database while you were trying to read or write to it.
    ///
    /// In SQLite parlance, this is called an ["expired
    /// blob"](https://sqlite.org/c3ref/blob_open.html).
    BlobExpired,

    /// Attempted to write more data to the SQLite archive than its maximum blob size will allow.
    FileTooBig,

    /// Attempted to write to a read-only database.
    ReadOnly,

    /// There was an error from the underlying SQLite database.
    Sqlite,

    /// An I/O error occurred.
    Io {
        /// The [`std::io::ErrorKind`] of the I/O error.
        kind: io::ErrorKind,
    },
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            // If you update one of these descriptions, you may also want to update the doc comment
            // on the `ErrorKind` variant.
            ErrorKind::AlreadyExists => "A resource already exists.",
            ErrorKind::NotFound => "A resource was not found.",
            ErrorKind::InvalidArgs => "Some arguments were invalid.",
            ErrorKind::CompressionNotSupported => "Attempted to read a compressed file, but the `deflate` Cargo feature was disabled.",
            ErrorKind::BlobExpired => "A file was modified in the database while you were trying to read or write to it.",
            ErrorKind::FileTooBig => "Attempted to write more data to the SQLite archive than its maximum blob size will allow.",
            ErrorKind::ReadOnly => "Attempted to write to a read-only database.",
            ErrorKind::Sqlite => "There was an error from the underlying SQLite database.",
            ErrorKind::Io { .. } => "An I/O error occurred.",
        })
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
