use super::metadata::FileMode;
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A file in a SQL archive.
#[derive(Debug)]
pub struct File {
    path: PathBuf,
    mode: FileMode,
    mtime: SystemTime,
    len: u64,
    cursor: u64,
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
        self.len
    }

    /// Whether the file is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Read for File {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        todo!()
    }
}

impl Write for File {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}

impl Seek for File {
    fn seek(&mut self, _pos: io::SeekFrom) -> io::Result<u64> {
        todo!()
    }
}
