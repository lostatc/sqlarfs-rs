use std::fmt;
use std::io::{self, Read};

#[cfg(feature = "deflate")]
use flate2::read::ZlibDecoder;
use rusqlite::blob::Blob;

use super::store::FileBlob;

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
    #[cfg(feature = "deflate")]
    pub const FAST: Self = Self::Deflate { level: 1 };

    /// Compression optimized for minimum output size.
    #[cfg(feature = "deflate")]
    pub const BEST: Self = Self::Deflate { level: 9 };
}

enum InnerReader<'conn> {
    #[cfg(feature = "deflate")]
    Compressed(ZlibDecoder<Blob<'conn>>),
    Uncompressed(Blob<'conn>),
}

impl<'conn> fmt::Debug for InnerReader<'conn> {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "deflate")]
            Self::Compressed(_) => f.debug_tuple("Compressed").finish(),
            Self::Uncompressed(_) => f.debug_tuple("Uncompressed").finish(),
        }
    }
}

impl<'conn> Read for InnerReader<'conn> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(feature = "deflate")]
            InnerReader::Compressed(reader) => reader.read(buf),
            InnerReader::Uncompressed(reader) => reader.read(buf),
        }
    }
}

/// A readable stream of the data in a [`File`].
///
/// This implements [`Read`] for reading a stream of data from a [`File`]. It does not support
/// seeking.
///
/// [`File`]: crate::File
#[derive(Debug)]
pub struct FileReader<'conn> {
    inner: InnerReader<'conn>,
}

impl<'conn> FileReader<'conn> {
    pub(super) fn new(blob: FileBlob<'conn>) -> crate::Result<Self> {
        if blob.is_compressed() {
            #[cfg(feature = "deflate")]
            return Ok(Self {
                inner: InnerReader::Compressed(ZlibDecoder::new(blob.into_blob())),
            });

            #[cfg(not(feature = "deflate"))]
            return Err(crate::Error::CompressionNotSupported);
        }

        Ok(Self {
            inner: InnerReader::Uncompressed(blob.into_blob()),
        })
    }
}

impl<'conn> Read for FileReader<'conn> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
