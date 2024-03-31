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

#[cfg(feature = "deflate")]
const COPY_BUF_SIZE: usize = 1024 * 8;

/// Metadata for a [`File`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    /// The time the file was last modified.
    ///
    /// This value has second precision.
    pub mtime: Option<SystemTime>,

    /// The file mode (permissions).
    pub mode: Option<FileMode>,

    /// The uncompressed size of the file.
    pub size: u64,
}

/// A file in a SQLite archive.
///
/// A [`File`] is a handle to a file that may or may not exist in the SQLite archive. Attempting to
/// read or write data or metadata on this file will generally return an error of
/// [`ErrorKind::NotFound`] if the file doesn't exist. You can use [`File::exists`] to check if it
/// exists, [`File::create`] to create it if it doesn't exist, and [`File::delete`] to delete it if
/// it does.
///
/// You can read from the beginning of a file, but cannot seek through it. You can truncate and
/// overwrite the file's contents, but cannot append to it.
///
/// Writing to a file does not automatically update its [`FileMetadata::mtime`].
///
/// Writes to a [`File`] can optionally be compressed with DEFLATE. You can change the compression
/// method (compressed or uncompressed) via [`File::set_compression`]. The default is to compress
/// writes if and only if the `deflate` Cargo feature is enabled. The selected compression method
/// does not affect the ability to read compressed files, but attempting to read a compressed file
/// will return an error if the `deflate` feature is disabled.
///
/// [`Read`]: std::io::Read
/// [`Write`]: std::io::Write
/// [`Seek`]: std::io::Seek
/// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
/// [`ErrorKind::CompressionNotSupported`]: crate::ErrorKind::CompressionNotSupported
#[derive(Debug)]
pub struct File<'conn, 'a> {
    path: PathBuf,
    compression: Compression,
    store: &'a mut Store<'conn>,
}

impl<'conn, 'a> File<'conn, 'a> {
    pub(super) fn new(path: PathBuf, store: &'a mut Store<'conn>) -> crate::Result<Self> {
        if path.is_absolute() {
            return Err(crate::ErrorKind::PathIsAbsolute.into());
        }

        Ok(Self {
            path,
            store,
            #[cfg(feature = "deflate")]
            compression: Compression::FAST,
            #[cfg(not(feature = "deflate"))]
            compression: Compression::None,
        })
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
        match self.metadata() {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == &crate::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(err),
        }
    }

    /// Create the file if it doesn't already exist.
    ///
    /// This does not set the file mode or mtime. You can set the file metadata using
    /// [`File::create_with`], or with [`File::set_mode`] and [`File::set_mtime`].
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::AlreadyExists`]: This file already exists in the archive.
    ///
    /// [`ErrorKind::AlreadyExists`]: crate::ErrorKind::AlreadyExists
    pub fn create(&mut self) -> crate::Result<()> {
        self.store.create_file(&self.path, None, None)
    }

    /// Create the file if it doesn't already exist and set its metadata.
    ///
    /// This accepts the initial file `mode` and `mtime`.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::AlreadyExists`]: This file already exists in the archive.
    ///
    /// [`ErrorKind::AlreadyExists`]: crate::ErrorKind::AlreadyExists
    pub fn create_with(&mut self, mode: FileMode, mtime: SystemTime) -> crate::Result<()> {
        self.store.create_file(&self.path, Some(mode), Some(mtime))
    }

    /// Delete the file from the archive.
    ///
    /// This does not consume its receiver and does not invalidate the file handle; you can still
    /// call [`File::create`] to create the file again.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn delete(&mut self) -> crate::Result<()> {
        self.store.delete_file(&self.path)
    }

    /// The file metadata.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn metadata(&self) -> crate::Result<FileMetadata> {
        self.store.read_metadata(&self.path)
    }

    /// Set the file mode.
    ///
    /// The file mode is nullable, so it's possible to set this to `None`.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn set_mode(&mut self, mode: Option<FileMode>) -> crate::Result<()> {
        self.store.set_mode(&self.path, mode)
    }

    /// Set the time the file was last modified.
    ///
    /// The file mtime is nullable, so it's possible to set this to `None`.
    ///
    /// This rounds to the nearest second.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn set_mtime(&mut self, mtime: Option<SystemTime>) -> crate::Result<()> {
        self.store.set_mtime(&self.path, mtime)
    }

    /// Whether the file is empty.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    pub fn is_empty(&self) -> crate::Result<bool> {
        let metadata = self.metadata()?;
        Ok(metadata.size == 0)
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
    // (e.g. mode or mtime) while the blob is open. This would generate an "expired blob" error.
    //
    // Read about expired blobs:
    // https://sqlite.org/c3ref/blob_open.html
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
                    //
                    // Additionally, we need to know whether the compressed data is smaller than
                    // the uncompressed data or not, but we want to avoid keeping both the full
                    // uncompressed data and the full compressed data in memory, because the
                    // `reader` could potentially return a large amount of data.
                    //
                    // This implementation tries to strike a balance between minimizing the amount
                    // of data we're keeping in memory and avoiding the need to do extra work
                    // compressing data multiple times.
                    //
                    // The worst-case scenario is that we find out the input is compressible only
                    // after we've compressed a lot of it, after which we end up compressing it
                    // again.
                    //
                    // However, if the input is compressible, we'll probably figure that out pretty
                    // quickly. As files get larger, the probability that they can't be compressed
                    // *at all* decreases.
                    //
                    // We're also relying on the user to disable compression if they know they're
                    // going to be writing a lot of data that's mostly incompressible (e.g. photos
                    // and videos that are already compressed).

                    let compression_level = flate2::Compression::new(level);

                    let allocation_size =
                        match size_hint {
                            Some(len) => Some(len.try_into().map_err(|err| {
                                crate::Error::new(crate::ErrorKind::FileTooBig, err)
                            })?),
                            None => None,
                        };

                    let mut uncompressed_buf = if let Some(capacity) = allocation_size {
                        Vec::with_capacity(capacity)
                    } else {
                        Vec::new()
                    };

                    let mut copy_buf = vec![0u8; COPY_BUF_SIZE];

                    // This encoder doesn't write the compressed data anywhere; we're only using it
                    // to determine the compressed size of the data.
                    let mut test_encoder = ZlibEncoder::new(io::sink(), compression_level);

                    let mut is_compressible = false;

                    // We need to keep track of the total uncompressed size of the input, because
                    // the uncompressed size of the file goes in the database.
                    let mut bytes_read_so_far = 0;

                    // Determine whether this file is compressible by writing the data to the
                    // encoder until it says the output size is smaller than the input size.
                    loop {
                        let bytes_read = reader.read(&mut copy_buf)?;
                        bytes_read_so_far += u64_from_usize(bytes_read);

                        if bytes_read == 0 {
                            break;
                        }

                        uncompressed_buf.extend_from_slice(&copy_buf[..bytes_read]);

                        test_encoder.write_all(&copy_buf[..bytes_read])?;

                        // Flush the encoder's internal buffer to ensure we get an accurate count
                        // of the total number of bytes input and output.
                        test_encoder.flush()?;

                        if test_encoder.total_out() < test_encoder.total_in() {
                            is_compressible = true;
                            break;
                        }
                    }

                    let bytes_to_write = if is_compressible {
                        // Now that we know the file is compressible, and we have the full contents
                        // of the `reader` in memory, we can compress it and keep the result to
                        // write to the blob.

                        let compressed_buf = if let Some(capacity) = allocation_size {
                            Vec::with_capacity(capacity)
                        } else {
                            Vec::new()
                        };

                        let mut encoder = ZlibEncoder::new(compressed_buf, compression_level);

                        // Copy the data we've read from the `reader` so far into the encoder.
                        encoder.write_all(&uncompressed_buf)?;

                        // Drop the uncompressed data to free that memory; we don't need it
                        // anymore.
                        drop(uncompressed_buf);

                        // Copy the rest of the data—the data we have not read yet—into the
                        // encoder.
                        bytes_read_so_far += io::copy(reader, &mut encoder)?;

                        encoder.finish()?
                    } else {
                        uncompressed_buf
                    };

                    store.allocate_blob(&self.path, u64_from_usize(bytes_to_write.len()))?;
                    let mut target_blob = store.open_blob(&self.path, false)?.into_blob();

                    target_blob.write_all(&bytes_to_write)?;

                    bytes_read_so_far
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

                    // Only use the compressed data if it's smaller than the uncompressed data. The
                    // sqlar spec requires this.
                    if compressed_bytes.len() < bytes.len() {
                        store.store_blob(&self.path, &compressed_bytes)?;
                    } else {
                        store.store_blob(&self.path, bytes)?;
                    }
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
    /// Prefer this to [`File::write_from`] if the input is a [`std::fs::File`].
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

    /// The current compression method used when writing to the file.
    pub fn compression(&mut self) -> Compression {
        self.compression
    }

    /// Set the compression method used when writing to the file.
    pub fn set_compression(&mut self, method: Compression) {
        self.compression = method;
    }
}
