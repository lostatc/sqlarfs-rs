use std::fmt;
use std::io::{self, Read};

use rusqlite::blob::Blob;

use super::error::io_err_has_sqlite_code;

/// The compression method to use when writing to a [`File`].
///
/// [`File`]: crate::File
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Compression {
    /// Do not compress writes.
    None,

    /// Compress writes using the DEFLATE algorithm.
    #[cfg(feature = "deflate")]
    Deflate {
        /// The compression level to use.
        ///
        /// This value is on a scale of 0-9, where 0 means "no compression" and 9 means "maximum
        /// compression."
        level: u32,
    },
}

impl Compression {
    /// Compression optimized for best speed of encoding.
    pub const FAST: Self = Self::Deflate { level: 1 };

    /// Compression optimized for minimum output size.
    pub const BEST: Self = Self::Deflate { level: 9 };
}

/// A readable stream of the data in a [`File`].
///
/// This implements [`Read`] for reading a stream of data from a [`File`], and does not support
/// seeking.
///
/// Attempting to read a file that has been modified out from under this reader will fail with
/// [`Error::BlobExpired`].
///
/// Attempting to read a compressed file will fail with [`Error::CompressionNotSupported`] if the
/// `deflate` Cargo feature is disabled.
///
/// [`File`]: crate::File
/// [`Error::BlobExpired`]: crate::BlobExpired
/// [`Error::CompressionNotSupported`]: crate::CompressionNotSupported
pub struct FileReader<'a> {
    blob: Blob<'a>,
}

impl<'a> fmt::Debug for FileReader<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileReader").finish_non_exhaustive()
    }
}

impl<'a> FileReader<'a> {
    pub(super) fn new(blob: Blob<'a>) -> Self {
        Self { blob }
    }
}

impl<'a> Read for FileReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.blob.read(buf).map_err(|err| {
            if io_err_has_sqlite_code(&err, rusqlite::ffi::ErrorCode::OperationAborted) {
                return crate::Error::BlobExpired.into();
            }

            err
        })
    }
}
