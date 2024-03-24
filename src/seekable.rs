use std::fmt;
use std::io::{self, Read, Seek, Write};

use rusqlite::blob::Blob;

use super::file::File;

/// A file in a SQLite archive that implements [`Read`], [`Write`], and [`Seek`].
pub struct SeekableFile<'a> {
    file: File<'a>,
    blob: Blob<'a>,
}

impl<'a> fmt::Debug for SeekableFile<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SeekableFile")
            .field("file", &self.file)
            .finish_non_exhaustive()
    }
}

impl<'a> SeekableFile<'a> {
    /// Return a reference to the underlying [`File`].
    pub fn as_file(&self) -> &File {
        &self.file
    }

    /// Return a mutable reference to the underlying [`File`].
    pub fn as_file_mut(&'a mut self) -> &mut File {
        &mut self.file
    }

    /// Consume this `SeekableFile` and return the underlying [`File`].
    pub fn into_file(self) -> File<'a> {
        self.file
    }
}

impl<'a> From<SeekableFile<'a>> for File<'a> {
    fn from(file: SeekableFile<'a>) -> Self {
        file.into_file()
    }
}

impl<'a> TryFrom<File<'a>> for SeekableFile<'a> {
    type Error = crate::Error;

    fn try_from(file: File<'a>) -> Result<Self, Self::Error> {
        file.into_seekable()
    }
}

impl<'a> AsRef<File<'a>> for SeekableFile<'a> {
    fn as_ref(&self) -> &File<'a> {
        &self.file
    }
}

impl<'a> AsMut<File<'a>> for SeekableFile<'a> {
    fn as_mut(&mut self) -> &mut File<'a> {
        &mut self.file
    }
}

impl<'a> Read for SeekableFile<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.blob.read(buf)
    }
}

impl<'a> Write for SeekableFile<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.blob.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.blob.flush()
    }
}

impl<'a> Seek for SeekableFile<'a> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.blob.seek(pos)
    }
}
