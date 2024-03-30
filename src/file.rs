use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(feature = "deflate")]
use flate2::write::ZlibEncoder;

use super::metadata::FileMode;
use super::store::Store;
use super::stream::{Compression, FileReader};
use super::util::u64_from_usize;

/// A file in a SQL archive.
///
/// Writes to a [`File`] can optionally be compressed with DEFLATE. You can change the compression
/// method (compressed or uncompressed) via [`File::set_compression`]. The default is to compress
/// writes if and only if the `deflate` Cargo feature is enabled. The selected compression method
/// does not affect the ability to read compressed files, but attempting to read a compressed file
/// will fail with [`ErrorKind::CompressionNotSupported`] if the `deflate` feature is disabled.
///
/// [`Read`]: std::io::Read
/// [`Write`]: std::io::Write
/// [`Seek`]: std::io::Seek
/// [`ErrorKind::CompressionNotSupported`]: crate::ErrorKind::CompressionNotSupported
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
    /// calling [`File::create`] and handling the potential [`ErrorKind::AlreadyExists`].
    ///
    /// [`ErrorKind::AlreadyExists`]: crate::ErrorKind::AlreadyExists
    pub fn exists(&self) -> crate::Result<bool> {
        todo!()
    }

    /// Create the file if it doesn't already exist.
    ///
    /// This accepts the initial [`FileMode`] of the file and sets the mtime to now.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::AlreadyExists`]: This file already exists in the archive.
    ///
    /// [`ErrorKind::AlreadyExists`]: crate::ErrorKind::AlreadyExists
    pub fn create(&mut self, mode: FileMode) -> crate::Result<()> {
        self.store
            .create_file(&self.path, Some(mode), Some(SystemTime::now()))
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
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn mode(&self) -> crate::Result<FileMode> {
        todo!()
    }

    /// Set the file mode.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn set_mode(&mut self, _mode: FileMode) -> crate::Result<()> {
        todo!()
    }

    /// The time the file was last modified.
    ///
    /// This value has second precision.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn mtime(&self) -> crate::Result<SystemTime> {
        todo!()
    }

    /// Set the time the file was last modified.
    ///
    /// This rounds to the nearest second.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn set_mtime(&mut self, _mtime: SystemTime) -> crate::Result<()> {
        todo!()
    }

    /// The uncompressed size of the file.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn len(&self) -> crate::Result<u64> {
        todo!()
    }

    /// Whether the file is empty.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn is_empty(&self) -> crate::Result<bool> {
        Ok(self.len()? == 0)
    }

    /// Truncate the file to zero bytes.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn truncate(&mut self) -> crate::Result<()> {
        self.store.exec(|store| {
            store.allocate_blob(&self.path, 0)?;
            store.set_size(&self.path, 0)?;

            Ok(())
        })
    }

    //
    // Opening a reader must take a mutable receiver to ensure that the user can't edit the row
    // (e.g. mode or mtime) while the blob is open. This would generate an expired blob error.
    //

    /// Get a readable stream of the data in the file.
    ///
    /// This starts reading from the beginning of the file. It does not support seeking.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    /// - [`ErrorKind::CompressionNotSupported`]: This file is compressed, but the `deflate` Cargo
    /// feature is disabled.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::CompressionNotSupported`]: crate::ErrorKind::CompressionNotSupported
    pub fn reader(&mut self) -> crate::Result<FileReader> {
        FileReader::new(self.store.open_blob(&self.path, true)?)
    }

    fn write_stream<R>(&mut self, reader: &mut R, size_hint: Option<u64>) -> crate::Result<()>
    where
        R: ?Sized + Read,
    {
        self.store.exec(|store| {
            let original_size = match self.compression {
                Compression::None => match size_hint {
                    Some(len) => {
                        // We have the length of the input stream, so we can allocate a blob in the
                        // database of that size and write to the database directly.

                        store.allocate_blob(&self.path, len)?;
                        let mut blob = store.open_blob(&self.path, false)?.into_blob();

                        io::copy(reader, &mut blob)?
                    }
                    None => {
                        // We do not have the length of the input stream, so we need to write it to
                        // an in-memory buffer to find out how large of a blob to allocate in the
                        // database.

                        let mut buf = Vec::new();
                        reader.read_to_end(&mut buf)?;

                        store.allocate_blob(&self.path, u64_from_usize(buf.len()))?;
                        let mut blob = store.open_blob(&self.path, false)?.into_blob();

                        blob.write_all(&buf)?;

                        u64_from_usize(buf.len())
                    }
                },

                #[cfg(feature = "deflate")]
                Compression::Deflate { level } => {
                    // We have no way of knowing the compressed size of the data until we actually
                    // compress it, so we need to write it to an in-memory buffer to find out how
                    // large of a blob to allocate in the database.

                    let buf =
                        match size_hint {
                            Some(len) => Vec::with_capacity(len.try_into().map_err(|err| {
                                crate::Error::new(crate::ErrorKind::FileTooBig, err)
                            })?),
                            None => Vec::new(),
                        };

                    let mut encoder = ZlibEncoder::new(buf, flate2::Compression::new(level));

                    let bytes_read = io::copy(reader, &mut encoder)?;

                    let compressed_data = encoder.finish()?;

                    store.allocate_blob(&self.path, u64_from_usize(compressed_data.len()))?;
                    let mut target_blob = store.open_blob(&self.path, false)?.into_blob();

                    target_blob.write_all(&compressed_data)?;

                    bytes_read
                }
            };

            store.set_size(&self.path, original_size)?;

            Ok(())
        })
    }

    /// Copy the contents of the given `reader` into the file.
    ///
    /// This truncates the file and copies the entire `reader` into it.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn write_from<R>(&mut self, reader: &mut R) -> crate::Result<()>
    where
        R: ?Sized + Read,
    {
        self.write_stream(reader, None)
    }

    /// Overwrite the file with the given bytes.
    ///
    /// This truncates the file and writes all of the given bytes to it.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn write_bytes(&mut self, bytes: &[u8]) -> crate::Result<()> {
        self.store.exec(|store| {
            match self.compression {
                Compression::None => {
                    store.store_blob(&self.path, bytes)?;
                }
                #[cfg(feature = "deflate")]
                Compression::Deflate { level } => {
                    let mut encoder = ZlibEncoder::new(
                        Vec::with_capacity(bytes.len()),
                        flate2::Compression::new(level),
                    );
                    encoder.write_all(bytes)?;
                    let compressed_bytes = encoder.finish()?;

                    store.store_blob(&self.path, &compressed_bytes)?;
                }
            };

            store.set_size(&self.path, u64_from_usize(bytes.len()))?;

            Ok(())
        })
    }

    /// Overwrite the file with the given string.
    ///
    /// This truncates the file and writes the entire string to it.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn write_str<S: AsRef<str>>(&mut self, s: S) -> crate::Result<()> {
        self.write_bytes(s.as_ref().as_bytes())
    }

    /// Copy the contents of the given `file` into this file.
    ///
    /// This truncates this file and copies the entire `file` into it.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn write_file(&mut self, file: &mut fs::File) -> crate::Result<()> {
        // We know the size of the file, which enabled some optimizations.
        let metadata = file.metadata()?;
        self.write_stream(file, Some(metadata.len()))
    }
}
