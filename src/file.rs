use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::metadata::FileMode;
use super::stream::{FileReader, FileWriter};

/// A file in a SQL archive.
#[derive(Debug, Clone)]
pub struct File {
    path: PathBuf,
    mode: FileMode,
    mtime: SystemTime,
    original_len: u64,
    compressed_len: u64,
}

impl File {
    /// The path of the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The file mode.
    pub fn mode(&self) -> FileMode {
        self.mode
    }

    /// The time the file was last modified.
    ///
    /// This value has second precision.
    pub fn mtime(&self) -> SystemTime {
        self.mtime
    }

    /// The uncompressed size of the file.
    pub fn len(&self) -> u64 {
        self.original_len
    }

    /// Whether the file is empty.
    pub fn is_empty(&self) -> bool {
        self.original_len == 0
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
    pub fn into_seekable(self) -> Option<SeekableFile> {
        if self.original_len == self.compressed_len {
            Some(SeekableFile {
                file: self,
                cursor: 0,
            })
        } else {
            None
        }
    }

    /// Get a readable stream of the data in the file.
    ///
    /// This starts reading from the beginning of the file.
    pub fn reader(&mut self) -> FileReader<'_> {
        FileReader::new(self)
    }

    /// Get a writer for writing data to the file.
    ///
    /// This truncates the file and starts writing from the beginning of the file.
    pub fn writer(&mut self) -> crate::Result<FileWriter<'_>> {
        self.set_len(0)?;
        Ok(FileWriter::new(self))
    }
}

/// A file in a SQLite archive that implements [`Read`], and [`Write`], and [`Seek`].
#[derive(Debug)]
pub struct SeekableFile {
    file: File,
    cursor: u64,
}

impl SeekableFile {
    /// Return a reference to the underlying [`File`].
    pub fn as_file(&self) -> &File {
        &self.file
    }

    /// Return a mutable reference to the underlying [`File`].
    pub fn as_file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    /// Consume this `SeekableFile` and return the underlying [`File`].
    pub fn into_file(self) -> File {
        self.file
    }
}

impl From<SeekableFile> for File {
    fn from(file: SeekableFile) -> Self {
        file.into_file()
    }
}

impl AsRef<File> for SeekableFile {
    fn as_ref(&self) -> &File {
        &self.file
    }
}

impl AsMut<File> for SeekableFile {
    fn as_mut(&mut self) -> &mut File {
        &mut self.file
    }
}

impl Read for SeekableFile {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        todo!()
    }
}

impl Write for SeekableFile {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}

impl Seek for SeekableFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let current_len = self.as_file().len();

        let new_pos = match pos {
            io::SeekFrom::Start(off) => Some(off),
            io::SeekFrom::End(off) => current_len.checked_add_signed(off),
            io::SeekFrom::Current(off) => self.cursor.checked_add_signed(off),
        };

        let new_pos = match new_pos {
            Some(new_pos) => new_pos,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Attempted to seek to a negative offset or integer overflow.",
                ))
            }
        };

        // If we seek past the end of the file, we need to fill the space with null bytes.
        if new_pos > current_len {
            self.as_file_mut().set_len(new_pos)?;
        }

        self.cursor = new_pos;

        Ok(self.cursor)
    }
}
