use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(feature = "deflate")]
use {flate2::write::ZlibEncoder, rand::prelude::*, rand::rngs::SmallRng};

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
/// will fail with [`Error::CompressionNotSupported`] if the `deflate` feature is disabled.
///
/// [`Read`]: std::io::Read
/// [`Write`]: std::io::Write
/// [`Seek`]: std::io::Seek
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
    /// - [`Error::NotFound`]: This file does not exist.
    /// - [`Error::CompressionNotSupported`]: This file is compressed, but the `deflate` Cargo
    /// feature is disabled.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn reader(&mut self) -> crate::Result<FileReader> {
        FileReader::new(self.store.open_blob(&self.path, true)?)
    }

    /// Copy the contents of the given `reader` into the file.
    ///
    /// This truncates the file and copies the entire `reader` into it.
    ///
    /// This accepts the number of bytes `len` that will be read from the `reader` and written to
    /// the file. You must know the number of bytes that will be written ahead of time because that
    /// space needs to be allocated in the database.
    ///
    /// This does not read the entire stream into memory. However, if you're using compression and
    /// the data you're trying to write will fit in memory, [`File::write_bytes`] may be more
    /// efficient.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn write_stream<R: Read>(&mut self, reader: R, len: u64) -> crate::Result<()> {
        self.store.exec(|store| {
            match self.compression {
                Compression::None => {
                    store.allocate_blob(&self.path, len)?;

                    let mut blob = store.open_blob(&self.path, false)?.into_blob();

                    io::copy(&mut reader.take(len), &mut blob)?;
                }

                // When using compression, we have to do ✨ weird stuff ✨ in order to avoid
                // reading the entire buffer into memory. This will almost certainly be less
                // efficient than doing this in memory, which is what `File::write_bytes` is for.
                #[cfg(feature = "deflate")]
                Compression::Deflate { level } => {
                    // Create a random string to act as the file path of the temporary row. This is
                    // long enough to make collisions practically impossible.
                    let rng = SmallRng::from_entropy();
                    let temp_filename: String = rng
                        .sample_iter(&rand::distributions::Alphanumeric)
                        .take(16)
                        .map(char::from)
                        .collect();
                    let temp_path = Path::new(&temp_filename);

                    // Create a temporary row in the sqlar table that we will write the compressed
                    // data to initially before copying it into its *actual* row.
                    store.create_file(temp_path, None, None)?;

                    // Allocate a blob with size equal to the length of the uncompressed data. This
                    // will be too big, but we won't know the compressed size of the data until we
                    // actually compress it. Once we know the compressed size, we'll copy just the
                    // filled portion of this blob into its actual row.
                    store.allocate_blob(temp_path, len)?;
                    let temp_blob = store.open_blob(temp_path, false)?.into_blob();

                    // Compress the data and write it to the temporary row.
                    let mut encoder = ZlibEncoder::new(temp_blob, flate2::Compression::new(level));
                    io::copy(&mut reader.take(len), &mut encoder)?;

                    // Get the compressed size of the data.
                    let mut temp_blob = encoder.finish()?;
                    let compressed_size = temp_blob.stream_position()?;

                    // Allocate a blob in the correct row (the row that corresponds to this `File`)
                    // that's the same size as the compressed data, so we're not wasting space and
                    // negating the benefits of compression.
                    store.allocate_blob(&self.path, compressed_size)?;
                    let mut target_blob = store.open_blob(&self.path, false)?.into_blob();

                    // Copy only the filled portion of this blob (not the trailing null bytes) from
                    // the temporary blob into the actual blob.
                    temp_blob.seek(io::SeekFrom::Start(0))?;
                    io::copy(&mut temp_blob.take(compressed_size), &mut target_blob)?;

                    // Finally, delete the temporary blob/row.
                    store.delete_file(temp_path)?;
                }
            };

            store.set_size(&self.path, len)?;

            Ok(())
        })
    }

    /// Overwrite the file with the given bytes.
    ///
    /// This truncates the file and writes all of the given bytes to it.
    ///
    /// If you're using compression and the data you're trying to write will fit in memory, this
    /// may be more efficient than [`File::write_stream`].
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
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
    /// - [`Error::NotFound`]: This file does not exist.
    ///
    /// [`Error::NotFound`]: crate::Error::NotFound
    pub fn write_str<S: AsRef<str>>(&mut self, s: S) -> crate::Result<()> {
        self.write_bytes(s.as_ref().as_bytes())
    }
}
