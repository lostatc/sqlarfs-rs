use std::fmt;
use std::io;
use std::result;

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

    /// Create a new [`Error`] from the given `message`.
    pub fn msg<M>(kind: ErrorKind, message: M) -> Self
    where
        M: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Self {
            kind,
            source: Some(anyhow::Error::msg(message)),
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
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|err| err.as_ref())
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
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn from(err: Error) -> Self {
        // Don't use a default match arm here. We want to be explicit about how we're mapping
        // `ErrorKind` variants to `io::ErrorKind` variants and make sure we remember to update
        // this when we add new ones.
        let kind = match err.kind() {
            ErrorKind::AlreadyExists => io::ErrorKind::AlreadyExists,
            ErrorKind::NotFound => io::ErrorKind::NotFound,
            ErrorKind::InvalidArgs => io::ErrorKind::InvalidInput,
            ErrorKind::NotARegularFile => io::ErrorKind::Other,
            // When it's stable, we can use `std::io::ErrorKind::NotADirectory`.
            ErrorKind::NotADirectory => io::ErrorKind::Other,
            // When it's stable, we can use `std::io::ErrorKind::FilesystemLoop`.
            ErrorKind::FilesystemLoop => io::ErrorKind::Other,
            ErrorKind::CompressionNotSupported => io::ErrorKind::InvalidInput,
            ErrorKind::FileTooBig => io::ErrorKind::Other,
            ErrorKind::ReadOnly => io::ErrorKind::Other,
            ErrorKind::CannotOpen => io::ErrorKind::Other,
            ErrorKind::NotADatabase => io::ErrorKind::Other,
            ErrorKind::Sqlite { .. } => io::ErrorKind::Other,
            ErrorKind::Io { kind } => *kind,
        };

        io::Error::new(kind, err)
    }
}

impl From<rusqlite::Error> for Error {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn from(err: rusqlite::Error) -> Self {
        let code = match err.sqlite_error() {
            Some(rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::ReadOnly,
                ..
            }) => ErrorKind::ReadOnly,
            Some(rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::TooBig,
                ..
            }) => ErrorKind::FileTooBig,
            Some(rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::CannotOpen,
                ..
            }) => ErrorKind::CannotOpen,
            Some(rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::NotADatabase,
                ..
            }) => ErrorKind::NotADatabase,
            code => ErrorKind::Sqlite {
                code: code.map(|code| SqliteErrorCode {
                    extended_code: code.extended_code,
                }),
            },
        };

        Self::new(code, err)
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

    // As a rule, we don't document this error kind as a possible error return in the API docs
    // because a) there may be several possible cases where it could be returned and b) it
    // generally represents an error on the part of the user and isn't useful to catch. We may
    // still document the circumstances that could lean to this error, but not that this specific
    // error kind would be returned.
    /// Some arguments were invalid.
    InvalidArgs,

    /// A file was unexpectedly a directory or a symbolic link.
    NotARegularFile,

    /// A file was unexpectedly not a directory.
    NotADirectory,

    /// A loop of symbolic links was encountered while traversing the filesystem.
    FilesystemLoop,

    /// Attempted to read a compressed file, but the `deflate` Cargo feature was disabled.
    CompressionNotSupported,

    /// Attempted to write more data to the SQLite archive than its maximum blob size will allow.
    FileTooBig,

    /// Attempted to write to a read-only database.
    ReadOnly,

    /// Could not open the database file.
    CannotOpen,

    /// The given file is not a SQLite database.
    NotADatabase,

    /// There was an error from the underlying SQLite database.
    Sqlite {
        /// The underlying SQLite error code, if there is one.
        code: Option<SqliteErrorCode>,
    },

    /// An I/O error occurred.
    Io {
        /// The [`std::io::ErrorKind`] of the I/O error.
        kind: io::ErrorKind,
    },
}

impl fmt::Display for ErrorKind {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            // If you update one of these descriptions, you may also want to update the doc comment
            // on the `ErrorKind` variant.
            ErrorKind::AlreadyExists => "A resource already exists.",
            ErrorKind::NotFound => "A resource was not found.",
            ErrorKind::InvalidArgs => "Some arguments were invalid.",
            ErrorKind::NotARegularFile => "A file was unexpectedly a directory or a symbolic link.",
            ErrorKind::NotADirectory => "A file was unexpectedly not a directory.",
            ErrorKind::FilesystemLoop => "A loop of symbolic links was encountered while traversing the filesystem.",
            ErrorKind::CompressionNotSupported => "Attempted to read a compressed file, but the `deflate` Cargo feature was disabled.",
            ErrorKind::FileTooBig => "Attempted to write more data to the SQLite archive than its maximum blob size will allow.",
            ErrorKind::ReadOnly => "Attempted to write to a read-only database.",
            ErrorKind::CannotOpen => "Could not open the database file.",
            ErrorKind::NotADatabase => "The given file is not a SQLite database.",
            ErrorKind::Sqlite { .. } => "There was an error from the underlying SQLite database.",
            ErrorKind::Io { .. } => "An I/O error occurred.",
        })
    }
}

/// The result type for sqlarfs.
pub type Result<T> = result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use anyhow::anyhow;
    use xpct::{be_ok, be_some, equal, expect};

    use super::*;

    #[test]
    fn get_error_kind() {
        let err = Error::new(
            ErrorKind::Io {
                kind: io::ErrorKind::Other,
            },
            io::Error::new(io::ErrorKind::Other, "inner error"),
        );

        expect!(err.kind()).to(equal(&ErrorKind::Io {
            kind: io::ErrorKind::Other,
        }));
        expect!(err.into_kind()).to(equal(ErrorKind::Io {
            kind: io::ErrorKind::Other,
        }));
    }

    #[test]
    fn get_wrapped_source_error() {
        let err = Error::new(
            ErrorKind::Io {
                kind: io::ErrorKind::Other,
            },
            io::Error::new(io::ErrorKind::Other, "inner error"),
        );

        expect!(err.source())
            .to(be_some())
            .map(|source| source.downcast_ref::<io::Error>())
            .to(be_some())
            .map(|err| err.kind())
            .to(equal(io::ErrorKind::Other));
    }

    #[test]
    fn convert_sqlarfs_io_err_into_std_io_error() {
        let err = Error::new(
            ErrorKind::Io {
                kind: io::ErrorKind::NotFound,
            },
            io::Error::new(io::ErrorKind::NotFound, "inner error"),
        );

        let io_err: io::Error = err.into();

        expect!(io_err.kind()).to(equal(io::ErrorKind::NotFound));

        expect!(io_err.into_inner())
            .to(be_some())
            .map(|err| err.downcast::<Error>())
            .to(be_ok())
            .map(|err| err.into_kind())
            .to(equal(ErrorKind::Io {
                kind: io::ErrorKind::NotFound,
            }));
    }

    #[test]
    fn convert_into_io_error_with_kind() {
        let err: Error = ErrorKind::NotFound.into();
        let io_err: io::Error = err.into();

        expect!(io_err.kind()).to(equal(io::ErrorKind::NotFound));
    }

    #[test]
    fn convert_from_io_error_with_kind() {
        let io_err: io::Error = io::ErrorKind::NotFound.into();
        let err: Error = io_err.into();

        expect!(err.kind()).to(equal(&ErrorKind::Io {
            kind: io::ErrorKind::NotFound,
        }));
    }

    #[test]
    fn convert_from_io_error_wrapping_a_sqlarfs_error() {
        let original_err: Error = ErrorKind::InvalidArgs.into();
        let io_err: io::Error = original_err.into();
        let unwrapped_error: Error = io_err.into();

        expect!(unwrapped_error.kind()).to(equal(&ErrorKind::InvalidArgs));
    }

    #[test]
    fn convert_from_io_error_wrapping_some_other_error() {
        let original_err = anyhow!("some error");
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, original_err);
        let unwrapped_error: Error = io_err.into();

        expect!(unwrapped_error.kind()).to(equal(&ErrorKind::Io {
            kind: io::ErrorKind::PermissionDenied,
        }));
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

        expect!(err.kind()).to(equal(&ErrorKind::ReadOnly));
    }
}
