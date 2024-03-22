use std::io::{self, Read, Seek, Write};

use super::file::File;

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
        let current_len = self.as_file().len()?;

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
