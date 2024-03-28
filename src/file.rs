use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::metadata::FileMode;
use super::store::Store;
use super::stream::{Compression, FileReader, FileWriter};
use super::util::u64_from_usize;

/// A file in a SQL archive.
///
/// Unless you have an exclusive lock on the database, it may be possible for other writers to
/// modify the file in the database out from under you. SQLite calls this situation an ["expired
/// blob"](https://sqlite.org/c3ref/blob_open.html), and it will cause reads and writes to return
/// an [`Error::BlobExpired`].
///
/// Writes to a [`File`] can optionally be compressed with DEFLATE. You can change the compression
/// method (compressed or uncompressed) via [`File::set_compression`]. The default is to compress
/// writes if and only if the `deflate` Cargo feature is enabled. The selected compression method
/// does not affect the ability to read compressed files, but attempting to read a compressed file
/// will fail with [`Error::CompressionNotSupported`].
///
/// [`Read`]: std::io::Read
/// [`Write`]: std::io::Write
/// [`Seek`]: std::io::Seek
/// [`Error::BlobExpired`]: crate::Error::BlobExpired
/// [`Error::CompressionNotSupported`]: crate::Error::CompressionNotSupported
#[derive(Debug)]
pub struct File<'conn, 'a> {
    path: PathBuf,
    compression: Compression,
    store: &'a mut Store<'conn>,
}

impl<'conn, 'a> File<'conn, 'a> {
    pub(super) fn new(path: PathBuf, store: &'a mut Store<'conn>) -> Self {
        Self {
            path,
            store,
            #[cfg(feature = "deflate")]
            compression: Compression::FAST,
            #[cfg(not(feature = "deflate"))]
            compression: Compression::None,
        }
    }

    //
    // Some operations, like setting the mode and mtime, don't strictly need to take a mutable
    // receiver. We make them take a mutable receiver anyways because:
    //
    // 1. The fact that we can implement this without mutable internal state is an implementation
    //    detail we don't need to expose.
    // 2. It gives users the option to have a read-only view of a file in a sqlar archive, which
    //    could be useful for maintaining certain invariants.
    //

    /// The path of the file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns whether the file actually exists in the database.
    ///
    /// Unless you have an exclusive lock on the database, the file may be deleted between when you
    /// call this method and when you act on its result! If you need the file to exist, consider
    /// calling [`File::create`] and handling the potential [`Error::AlreadyExists`].
    ///
    /// [`Error::AlreadyExists`]: crate::Error::AlreadyExists
    pub fn exists(&self) -> crate::Result<bool> {
        todo!()
    }

    /// Create the file if it doesn't already exist.
    ///
    /// This accepts the initial [`FileMode`] of the file and sets the mtime to now.
    ///
    /// # Errors
    ///
    /// - [`Error::AlreadyExists`]: This file already exists in the archive.
    ///
    /// [`Error::AlreadyExists`]: crate::Error::AlreadyExists
    pub fn create(&mut self, mode: FileMode) -> crate::Result<()> {
        self.store.create_file(&self.path, mode, SystemTime::now())
    }

    /// The current compression method used when writing to the file.
    pub fn compression(&mut self) -> Compression {
        self.compression
    }

    /// Set the compression method used when writing to the file.
    pub fn set_compression(&mut self, method: Compression) {
        self.compression = method;
    }

    /// The file mode.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn mode(&self) -> crate::Result<FileMode> {
        todo!()
    }

    /// Set the file mode.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn set_mode(&mut self, _mode: FileMode) -> crate::Result<()> {
        todo!()
    }

    /// The time the file was last modified.
    ///
    /// This value has second precision.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn mtime(&self) -> crate::Result<SystemTime> {
        todo!()
    }

    /// Set the time the file was last modified.
    ///
    /// This rounds to the nearest second.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn set_mtime(&mut self, _mtime: SystemTime) -> crate::Result<()> {
        todo!()
    }

    /// The uncompressed size of the file.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn len(&self) -> crate::Result<u64> {
        todo!()
    }

    /// Whether the file is empty.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn is_empty(&self) -> crate::Result<bool> {
        Ok(self.len()? == 0)
    }

    /// Truncate the file to zero bytes.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn truncate(&mut self) -> crate::Result<()> {
        self.store.truncate_blob(&self.path, 0)
    }

    //
    // Opening a reader or writer must take a mutable receiver to ensure that the user can't edit
    // the row (e.g. mode or mtime) while the blob is open. This would generate an expired blob
    // error.
    //

    /// Get a readable stream of the data in the file.
    ///
    /// This starts reading from the beginning of the file an does not support seeking.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn reader(&mut self) -> crate::Result<FileReader> {
        Ok(FileReader::new(
            self.store.open_blob(&self.path, false)?.into_blob(),
        ))
    }

    /// Get a writer for writing data to the file.
    ///
    /// This truncates the file and starts writing from the beginning of the file.
    ///
    /// This accepts the expected `len` of the file, which is the number of bytes that will be
    /// allocated in the database for it. If you end up writing fewer than `len` bytes, the
    /// remainder of the file will be filled with null bytes. This means that you generally only
    /// want to write to a file when you know the size of the input ahead of time.
    ///
    /// See these methods as well:
    ///
    /// - [`File::write_bytes`]
    /// - [`File::write_str`]
    /// - [`File::write_file`]
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn writer(&mut self, len: u64) -> crate::Result<FileWriter> {
        self.store.truncate_blob(&self.path, len)?;

        Ok(FileWriter::new(
            self.store.open_blob(&self.path, false)?.into_blob(),
            self.compression,
        ))
    }

    /// Overwrite the file with the given bytes.
    ///
    /// This truncates the file and writes all of the given bytes to it.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn write_bytes(&mut self, bytes: &[u8]) -> crate::Result<()> {
        self.store.exec(|store| {
            store.truncate_blob(&self.path, u64_from_usize(bytes.len()))?;

            let mut blob = store.open_blob(&self.path, false)?.into_blob();

            blob.write_all(bytes)?;

            Ok(())
        })?;

        Ok(())
    }

    /// Overwrite the file with the given string.
    ///
    /// This truncates the file and writes the entire string to it.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn write_str<S: AsRef<str>>(&mut self, s: S) -> crate::Result<()> {
        self.store.exec(|store| {
            let bytes = s.as_ref().as_bytes();

            store.truncate_blob(&self.path, u64_from_usize(bytes.len()))?;

            let mut blob = store.open_blob(&self.path, false)?.into_blob();

            blob.write_all(bytes)?;

            Ok(())
        })?;

        Ok(())
    }

    /// Copy the given `file` from the filesystem into this file.
    ///
    /// This truncates this file and writes the entire contents of the given `file` to it.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn write_file(&mut self, file: &mut fs::File) -> crate::Result<()> {
        self.store.exec(|store| {
            let metadata = file.metadata()?;

            store.truncate_blob(&self.path, metadata.len())?;

            let mut blob = store.open_blob(&self.path, false)?.into_blob();

            io::copy(file, &mut blob)?;

            Ok(())
        })?;

        Ok(())
    }
}
