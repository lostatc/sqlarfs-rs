use std::fmt;
use std::io::{self, Read, Seek, Write};

use rusqlite::blob::Blob;

use super::error::io_err_has_sqlite_code;
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

    /// Truncate or extend the file to the given `len`.
    ///
    /// If the given `len` is greater than the current size of the file, the file will be extended
    /// to `len` and the intermediate space will be filled with null bytes. This **does not**
    /// create a sparse hole in the file, as sqlar archives do not support sparse files.
    ///
    /// If `len` is less than the current size of the file and the seek position is past the point
    /// which the file is truncated to, it is moved to the new end of the file.
    pub fn set_len(&mut self, _len: u64) -> crate::Result<()> {
        todo!()
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
        self.blob.read(buf).map_err(|err| {
            if io_err_has_sqlite_code(&err, rusqlite::ffi::ErrorCode::OperationAborted) {
                return crate::Error::BlobExpired.into();
            }

            err
        })
    }
}

impl<'a> Write for SeekableFile<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.blob.write(buf).map_err(|err| {
            if io_err_has_sqlite_code(&err, rusqlite::ffi::ErrorCode::OperationAborted) {
                return crate::Error::BlobExpired.into();
            }

            err
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        self.blob.flush().map_err(|err| {
            if io_err_has_sqlite_code(&err, rusqlite::ffi::ErrorCode::OperationAborted) {
                return crate::Error::BlobExpired.into();
            }

            err
        })
    }
}

impl<'a> Seek for SeekableFile<'a> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.blob.seek(pos).map_err(|err| {
            if io_err_has_sqlite_code(&err, rusqlite::ffi::ErrorCode::OperationAborted) {
                return crate::Error::BlobExpired.into();
            }

            err
        })
    }
}
