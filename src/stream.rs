use std::fmt;
use std::io::{self, Read, Write};

use rusqlite::blob::Blob;

/// A readable stream of the data in a [`File`].
///
/// This implements [`Read`] for reading a stream of data from a [`File`], but does not support
/// seeking like [`SeekableFile`] does. You must use this over [`SeekableFile`] when the file is
/// compressed.
///
/// [`File`]: crate::File
/// [`SeekableFile`]: crate::SeekableFile
pub struct FileReader<'a> {
    blob: Blob<'a>,
}

impl<'a> fmt::Debug for FileReader<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileReader").finish_non_exhaustive()
    }
}

impl<'a> FileReader<'a> {
    pub fn new(blob: Blob<'a>) -> Self {
        Self { blob }
    }
}

impl<'a> Read for FileReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.blob.read(buf)
    }
}

/// A writer for writing data to a [`File`].
///
/// This implements [`Write`] for writing data to a [`File`], but does not support seeking like
/// [`SeekableFile`] does. You must use this over [`SeekableFile`] when the file is compressed.
///
/// [`File`]: crate::File
/// [`SeekableFile`]: crate::SeekableFile
pub struct FileWriter<'a> {
    blob: Blob<'a>,
}

impl<'a> fmt::Debug for FileWriter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileWriter").finish_non_exhaustive()
    }
}

impl<'a> FileWriter<'a> {
    pub fn new(blob: Blob<'a>) -> Self {
        Self { blob }
    }
}

impl<'a> Write for FileWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.blob.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.blob.flush()
    }
}
