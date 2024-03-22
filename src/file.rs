use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::metadata::FileMode;
use super::seekable::SeekableFile;
use super::stream::{FileReader, FileWriter};

/// A file in a SQL archive.
#[derive(Debug, Clone)]
pub struct File {
    path: PathBuf,
}

impl File {
    /// The path of the file.
    pub fn path(&self) -> &Path {
        &self.path
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
    /// compressed, this returns `None`. For compressed files, you can use [`FileReader`] and
    /// [`FileWriter`] instead.
    pub fn into_seekable(self) -> crate::Result<SeekableFile> {
        todo!()
    }

    /// Get a readable stream of the data in the file.
    ///
    /// This starts reading from the beginning of the file.
    pub fn reader(&mut self) -> crate::Result<FileReader<'_>> {
        todo!()
    }

    /// Get a writer for writing data to the file.
    ///
    /// This truncates the file and starts writing from the beginning of the file.
    pub fn writer(&mut self) -> crate::Result<FileWriter<'_>> {
        todo!()
    }
}
