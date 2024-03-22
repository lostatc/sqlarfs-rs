use super::metadata::FileMode;
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A file in a SQL archive.
#[derive(Debug, Clone)]
pub struct File {
    path: PathBuf,
    mode: FileMode,
    mtime: SystemTime,
    original_len: u64,
    compressed_len: u64,
}

// Keep these method doc comments in sync with `SeekableFile`.
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
}

/// A file in a SQLite archive that implements [`Read`], and [`Write`], and [`Seek`].
#[derive(Debug)]
pub struct SeekableFile {
    file: File,
    cursor: u64,
}

impl AsRef<File> for SeekableFile {
    fn as_ref(&self) -> &File {
        &self.file
    }
}

// Keep these method doc comments in sync with `File`.
impl SeekableFile {
    /// The path of the file.
    pub fn path(&self) -> &Path {
        &self.file.path
    }

    /// The file mode.
    pub fn mode(&self) -> FileMode {
        self.file.mode
    }

    /// The time the file was last modified.
    ///
    /// This value has second precision.
    pub fn mtime(&self) -> SystemTime {
        self.file.mtime
    }

    /// The uncompressed size of the file.
    pub fn len(&self) -> u64 {
        self.file.original_len
    }

    /// Whether the file is empty.
    pub fn is_empty(&self) -> bool {
        self.file.original_len == 0
    }

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
    fn seek(&mut self, _pos: io::SeekFrom) -> io::Result<u64> {
        todo!()
    }
}
