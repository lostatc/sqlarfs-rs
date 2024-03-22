use std::io::{self, Read, Write};

use crate::File;

/// A readable stream of the data in a [`File`].
///
/// This implements [`Read`] for reading a stream of data from a [`File`], but does not support
/// seeking. You must use this over [`SeekableFile`][crate::SeekableFile] when the file is
/// compressed.
#[derive(Debug)]
pub struct FileReader<'a> {
    file: &'a mut File,
}

impl<'a> FileReader<'a> {
    /// Return a reference to the underlying [`File`].
    pub fn as_file(&self) -> &File {
        self.file
    }
}

impl<'a> AsRef<File> for FileReader<'a> {
    fn as_ref(&self) -> &File {
        self.file
    }
}

impl<'a> Read for FileReader<'a> {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        todo!()
    }
}

/// A writer for writing data to a [`File`].
///
/// This implements [`Write`] for writing data to a [`File`], but does not support seeking. You
/// must use this over [`SeekableFile`][crate::SeekableFile] when the file is compressed.
#[derive(Debug)]
pub struct FileWriter<'a> {
    file: &'a mut File,
}

impl<'a> FileWriter<'a> {
    /// Return a reference to the underlying [`File`].
    pub fn as_file(&self) -> &File {
        self.file
    }
}

impl<'a> AsRef<File> for FileWriter<'a> {
    fn as_ref(&self) -> &File {
        self.file
    }
}

impl<'a> Write for FileWriter<'a> {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}
