use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::metadata::FileMode;
use super::seekable::SeekableFile;
use super::stream::{FileReader, FileWriter};

/// A file in a SQL archive.
///
///
/// If the file is uncompressed, you can get a [`SeekableFile`] with [`File::into_seekable`].
/// [`SeekableFile`] implements [`Read`], [`Write`], and [`Seek`].
///
/// If the file is compressed, your options are:
///
/// - Start reading the file from the beginning using [`File::reader`].
/// - Truncate the file and start writing using [`File::writer`].
///
/// Unless you have an exclusive lock on the database (see [`Archive::transaction_with`]), it may
/// be possible for other writers to modify the file in the database out from under you. SQLite
/// calls this situation an ["expired blob"](https://sqlite.org/c3ref/blob_open.html), and it will
/// cause reads and writes to return an [`Error::BlobExpired`].
///
/// [`Read`]: std::io::Read
/// [`Write`]: std::io::Write
/// [`Seek`]: std::io::Seek
/// [`Archive::transaction_with`]: crate::Archive::transaction_with
/// [`Error::BlobExpired`]: crate::Error::BlobExpired
#[derive(Debug, Clone)]
pub struct File<'a> {
    path: PathBuf,
    _conn: &'a rusqlite::Connection,
}

impl<'a> File<'a> {
    pub(super) fn new(path: PathBuf, conn: &'a rusqlite::Connection) -> Self {
        Self { path, _conn: conn }
    }

    /// The path of the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns whether this file actually exists in the database.
    ///
    /// ⚠️ Unless you have an exclusive lock on the database, the file may be deleted between when
    /// you call this method and when you act on its result!
    pub fn exists(&self) -> crate::Result<bool> {
        todo!()
    }

    /// The file mode.
    pub fn mode(&self) -> crate::Result<FileMode> {
        todo!()
    }

    /// Set the file mode.
    pub fn set_mode(&self, _mode: FileMode) -> crate::Result<()> {
        todo!()
    }

    /// The time the file was last modified.
    ///
    /// This value has second precision.
    pub fn mtime(&self) -> crate::Result<SystemTime> {
        todo!()
    }

    /// Set the time the file was last modified.
    ///
    /// This rounds to the nearest second.
    pub fn set_mtime(&self, _mtime: SystemTime) -> crate::Result<()> {
        todo!()
    }

    /// The uncompressed size of the file.
    pub fn len(&self) -> crate::Result<u64> {
        todo!()
    }

    /// Whether the file is empty.
    pub fn is_empty(&self) -> crate::Result<bool> {
        Ok(self.len()? == 0)
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

    /// Convert this file into a [`SeekableFile`].
    ///
    /// A [`SeekableFile`] implements [`Read`], [`Write`], and [`Seek`] for reading and writing the
    /// data in the file.
    ///
    /// You can only convert an uncompressed file into a [`SeekableFile`]. If the file is
    /// compressed, this returns [`Error::NotSeekable`]. For compressed files, you can use
    /// [`File::reader`] and [`File::writer`] instead.
    ///
    /// [`Read`]: std::io::Read
    /// [`Write`]: std::io::Write
    /// [`Seek`]: std::io::Seek
    /// [`Error::NotSeekable`]: crate::Error::NotSeekable
    pub fn into_seekable(self) -> crate::Result<SeekableFile<'a>> {
        todo!()
    }

    /// Get a readable stream of the data in the file.
    ///
    /// This starts reading from the beginning of the file.
    pub fn reader(&self) -> crate::Result<FileReader<'a>> {
        todo!()
    }

    /// Get a writer for writing data to the file.
    ///
    /// This truncates the file and starts writing from the beginning of the file.
    pub fn writer(&self) -> crate::Result<FileWriter<'a>> {
        todo!()
    }
}
