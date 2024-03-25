use std::fmt;
use std::io::{self, Read, Seek, Write};

use rusqlite::blob::Blob;

use super::error::io_err_has_sqlite_code;

/// A file in a SQLite archive that implements [`Read`], [`Write`], and [`Seek`].
pub struct SeekableFile<'a> {
    blob: Blob<'a>,
}

impl<'a> fmt::Debug for SeekableFile<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SeekableFile").finish_non_exhaustive()
    }
}

impl<'a> SeekableFile<'a> {
    pub(super) fn new(blob: Blob<'a>) -> Self {
        Self { blob }
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
