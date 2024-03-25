use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::db::Store;
use super::metadata::FileMode;
use super::seekable::SeekableFile;
use super::stream::{FileReader, FileWriter};

/// A file in a SQL archive.
///
///
/// If the file is uncompressed, you can get a [`SeekableFile`] with [`File::seekable`].
/// [`SeekableFile`] implements [`Read`], [`Write`], and [`Seek`].
///
/// If the file is compressed, your options are:
///
/// - Start reading the file from the beginning using [`File::reader`].
/// - Truncate the file and start writing using [`File::writer`].
///
/// Unless you have an exclusive lock on the database, it may be possible for other writers to
/// modify the file in the database out from under you. SQLite calls this situation an ["expired
/// blob"](https://sqlite.org/c3ref/blob_open.html), and it will cause reads and writes to return
/// an [`Error::BlobExpired`].
///
/// [`Read`]: std::io::Read
/// [`Write`]: std::io::Write
/// [`Seek`]: std::io::Seek
/// [`Error::BlobExpired`]: crate::Error::BlobExpired
#[derive(Debug)]
pub struct File<'a> {
    path: PathBuf,
    store: &'a Store<'a>,
}

impl<'a> File<'a> {
    pub(super) fn new(path: PathBuf, store: &'a Store<'a>) -> Self {
        Self { path, store }
    }

    //
    // Some operations, like setting the mode and mtime, don't strictly need to take a mutable
    // receiver. We make them take a mutable receiver anyways because:
    //
    // 1. The fact that we can implement this without mutable internal state as an implementation
    //    detail we don't need to expose.
    // 2. It gives users the option to have a read-only view of a file in a sqlar archive, which
    //    could be useful for maintaining certain invariants.
    //

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
    pub fn set_mode(&mut self, _mode: FileMode) -> crate::Result<()> {
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
    pub fn set_mtime(&mut self, _mtime: SystemTime) -> crate::Result<()> {
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

    /// Truncate this file to zero bytes.
    ///
    /// This moves the seek position back to the beginning of the file.
    ///
    /// If the file is seekable (not compressed), you can also use [`File::set_len`].
    pub fn truncate(&mut self) -> crate::Result<()> {
        todo!()
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

    //
    // Opening a seekable file, reader, or writer must take a mutable receiver to ensure that the
    // user can't edit the row (e.g. mode or mtime) while the blob is open. This would generate an
    // expired blob error.
    //

    /// Get a [`SeekableFile`] for reading and writing the contents of the file.
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
    pub fn seekable(&'a mut self) -> crate::Result<SeekableFile<'a>> {
        Ok(SeekableFile::new(self.store.open_blob(&self.path, false)?))
    }

    /// Get a readable stream of the data in the file.
    ///
    /// This starts reading from the beginning of the file.
    pub fn reader(&'a mut self) -> crate::Result<FileReader<'a>> {
        Ok(FileReader::new(self.store.open_blob(&self.path, false)?))
    }

    /// Get a writer for writing data to the file.
    ///
    /// This truncates the file and starts writing from the beginning of the file.
    pub fn writer(&'a mut self) -> crate::Result<FileWriter<'a>> {
        // TODO: Truncate the file first.
        Ok(FileWriter::new(self.store.open_blob(&self.path, false)?))
    }
}
