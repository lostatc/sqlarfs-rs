use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(feature = "deflate")]
use flate2::write::ZlibEncoder;

use super::metadata::{mode_from_umask, FileMetadata, FileMode, FileType};
use super::store::Store;
use super::stream::{Compression, FileReader};
use super::util::u64_from_usize;

#[cfg(feature = "deflate")]
const COPY_BUF_SIZE: usize = 1024 * 8;

fn unwrap_path_parent(path: &Path) -> &Path {
    path.parent().expect("The given file path is an absolute path, but we should have already checked for this when opening the file handle. This is a bug.")
}

/// A file in a SQLite archive.
///
/// A [`File`] is a handle to a file that may or may not exist in the SQLite archive. You can call
/// [`File::create_file`] or [`File::create_dir`] to actually create the file if it doesn't already
/// exist. Attempting to read or write data or metadata on this file will return an error if the
/// file doesn't exist.
///
/// # Reading and writing
///
/// You can read from the beginning of a file, but cannot seek through it. You can truncate and
/// overwrite the file's contents, but cannot append to it.
///
/// Writing to a file does not automatically update its [`FileMetadata::mtime`].
///
/// Attempting to read from or write to a directory will return an error.
///
/// # Compression
///
/// Writes to a [`File`] can optionally be compressed with DEFLATE. You can change the compression
/// method (compressed or uncompressed) via [`File::set_compression`]. The default is to compress
/// writes if and only if the `deflate` Cargo feature is enabled. You can read compressed files
/// regardless of the selected compression method, but doing so will return an error if the
/// `deflate` feature is disabled.
///
/// Consider disabling compression if you know you're going to be writing a lot of incompressible
/// data, such as files that are already compressed (e.g. photos and videos).
///
/// [`Read`]: std::io::Read
/// [`Write`]: std::io::Write
/// [`Seek`]: std::io::Seek
/// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
/// [`ErrorKind::CompressionNotSupported`]: crate::ErrorKind::CompressionNotSupported
#[derive(Debug)]
pub struct File<'conn, 'ar> {
    // We store this internally as a string because the contract of this type requires the path to
    // be valid Unicode, which `PathBuf` does not guarantee.
    path: String,
    compression: Compression,
    umask: FileMode,
    store: &'ar mut Store<'conn>,
}

impl<'conn, 'ar> File<'conn, 'ar> {
    pub(super) fn new(
        path: &Path,
        store: &'ar mut Store<'conn>,
        umask: FileMode,
    ) -> crate::Result<Self> {
        if path == Path::new("") {
            return Err(crate::Error::msg(
                crate::ErrorKind::InvalidArgs,
                "The given path is empty.",
            ));
        }

        if path.is_absolute() {
            return Err(crate::Error::msg(crate::ErrorKind::InvalidArgs, "The given path is an absolute path, but SQLite archives only support relative paths."));
        }

        let normalized_path = match path.as_os_str().to_str() {
            // SQLite archives created by the reference implementation don't have trailing slashes
            // in directory paths, so we normalize paths coming in by stripping trailing path
            // separators.
            Some(utf8_str) => utf8_str
                .trim_end_matches(std::path::MAIN_SEPARATOR)
                .to_owned(),
            None => {
                return Err(crate::Error::msg(
                    crate::ErrorKind::InvalidArgs,
                    "The given path is not valid Unicode.",
                ))
            }
        };

        Ok(Self {
            path: normalized_path,
            store,
            #[cfg(feature = "deflate")]
            compression: Compression::FAST,
            // Because getting a file handle requires a mutable receiver, we don't have to worry
            // about keeping this in sync with `Archive::umask`.
            umask,
            #[cfg(not(feature = "deflate"))]
            compression: Compression::None,
        })
    }

    fn validate_is_writable(&self) -> crate::Result<()> {
        if self.store.has_dir_mode(&self.path)? {
            Err(crate::Error::msg(
                crate::ErrorKind::IsADirectory,
                "Cannot write to a file with a directory mode.",
            ))
        } else if self.store.has_descendants(&self.path)? {
            Err(crate::Error::msg(
                crate::ErrorKind::IsADirectory,
                "Cannot write to a file that has descendants in the archive (i.e. is a directory).",
            ))
        } else {
            Ok(())
        }
    }

    fn validate_is_readable(&self) -> crate::Result<()> {
        if self.store.has_dir_mode(&self.path)? {
            Err(crate::Error::msg(
                crate::ErrorKind::IsADirectory,
                "Cannot read from a file with a directory mode.",
            ))
        } else if self.store.has_descendants(&self.path)? {
            Err(crate::Error::msg(
                crate::ErrorKind::IsADirectory,
                "Cannot read from a file that has descendants in the archive (i.e. is a directory).",
            ))
        } else {
            Ok(())
        }
    }

    fn validate_can_be_created(&self) -> crate::Result<()> {
        let parent_path = unwrap_path_parent(Path::new(&self.path));

        if parent_path == Path::new("") {
            // The path is a relative path with one component, meaning it doesn't have a parent and
            // is safe to create.
            return Ok(());
        }

        let parent_str = match parent_path.to_str() {
            Some(path) => path,
            None => panic!("The given path is not valid Unicode, but we should have already checked for this when opening the file handle. This is a bug."),
        };

        let parent_exists = match self.store.read_metadata(parent_str) {
            Ok(_) => true,
            Err(err) if err.kind() == &crate::ErrorKind::NotFound => false,
            Err(err) => return Err(err),
        };

        if !parent_exists {
            Err(crate::Error::msg(
                crate::ErrorKind::NotFound,
                "Cannot create a file whose parent does not exist.",
            ))
        } else if !self.store.has_dir_mode(parent_str)? {
            Err(crate::Error::msg(
                crate::ErrorKind::NotADirectory,
                "Cannot create a file whose parent does not have the directory mode.",
            ))
        } else if self.store.has_nonzero_size_ancestor(&self.path)? {
            Err(crate::Error::msg(
                crate::ErrorKind::NotADirectory,
                "Cannot create a file whose parent has a nonzero size (i.e. is not a directory).",
            ))
        } else {
            Ok(())
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
        Path::new(&self.path)
    }

    /// Returns whether the file actually exists in the database.
    ///
    /// Unless you have an exclusive lock on the database, the file may be deleted between when you
    /// call this method and when you act on its result! If you need the file to exist, consider
    /// creating the file and handling the potential [`ErrorKind::AlreadyExists`].
    ///
    /// [`ErrorKind::AlreadyExists`]: crate::ErrorKind::AlreadyExists
    pub fn exists(&self) -> crate::Result<bool> {
        match self.metadata() {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == &crate::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(err),
        }
    }

    /// Create a regular file if it doesn't already exist.
    ///
    /// This sets the file mode based on the current [`File::umask`] and sets the mtime to now. You
    /// can change the file metadata with [`File::set_mode`] and [`File::set_mtime`].
    ///
    /// # See also
    ///
    /// - [`File::create_dir`] to create a directory.
    /// - [`File::create_dir_all`] to create a directory and all its parent directories.
    /// - [`File::create_with`] to specify the metadata on file creation.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::AlreadyExists`]: This file already exists in the archive.
    /// - [`ErrorKind::NotFound`]: This file's parent directory does not exist.
    /// - [`ErrorKind::NotADirectory`]: The file's parent is not a directory.
    ///
    /// [`ErrorKind::AlreadyExists`]: crate::ErrorKind::AlreadyExists
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::NotADirectory`]: crate::ErrorKind::NotADirectory
    pub fn create_file(&mut self) -> crate::Result<()> {
        self.validate_can_be_created()?;

        self.store.create_file(
            &self.path,
            FileType::File,
            mode_from_umask(FileType::File, self.umask),
            Some(SystemTime::now()),
        )
    }

    /// Create a directory if it doesn't already exist.
    ///
    /// This sets the file mode based on the current [`File::umask`] and sets the mtime to now. You
    /// can change the file metadata with [`File::set_mode`] and [`File::set_mtime`].
    ///
    /// # See also
    ///
    /// - [`File::create_file`] to create a regular file.
    /// - [`File::create_dir_all`] to create a directory and all its parent directories.
    /// - [`File::create_with`] to specify the metadata on file creation.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::AlreadyExists`]: This file already exists in the archive.
    /// - [`ErrorKind::NotFound`]: This file's parent directory does not exist.
    /// - [`ErrorKind::NotADirectory`]: The file's parent is not a directory.
    ///
    /// [`ErrorKind::AlreadyExists`]: crate::ErrorKind::AlreadyExists
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::NotADirectory`]: crate::ErrorKind::NotADirectory
    pub fn create_dir(&mut self) -> crate::Result<()> {
        self.validate_can_be_created()?;

        self.store.create_file(
            &self.path,
            FileType::Dir,
            mode_from_umask(FileType::Dir, self.umask),
            Some(SystemTime::now()),
        )
    }

    /// Create a directory and all its missing parent directories.
    ///
    /// Unlike [`File::create_dir`], this does not return an error if the directory already exists.
    ///
    /// This sets the file mode based on the current [`File::umask`] and sets the mtime to now. You
    /// can change the file metadata with [`File::set_mode`] and [`File::set_mtime`].
    ///
    /// # See also
    ///
    /// - [`File::create_file`] to create a regular file.
    /// - [`File::create_dir`] to create a directory.
    /// - [`File::create_with`] to specify the metadata on file creation.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::AlreadyExists`]: This file already exists in the archive and is not a
    /// directory.
    /// - [`ErrorKind::NotADirectory`]: The file's parent is not a directory.
    ///
    /// [`ErrorKind::AlreadyExists`]: crate::ErrorKind::AlreadyExists
    /// [`ErrorKind::NotADirectory`]: crate::ErrorKind::NotADirectory
    pub fn create_dir_all(&mut self) -> crate::Result<()> {
        match self.validate_can_be_created() {
            Ok(_) => {}
            Err(err) if err.kind() == &crate::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }

        match self.metadata() {
            Ok(metadata) if !metadata.is_dir() => {
                return Err(crate::Error::msg(
                    crate::ErrorKind::AlreadyExists,
                    "This file already exists, but is not a directory.",
                ))
            }
            Ok(_) => {}
            Err(err) if err.kind() == &crate::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }

        let path = PathBuf::from(&self.path);
        let mode = mode_from_umask(FileType::Dir, self.umask);
        // Each parent directory should have the same mtime.
        let mtime = SystemTime::now();

        let mut parents = Vec::new();
        let mut parent = path.as_path();

        while parent != Path::new("") {
            parents.push(parent);
            parent = unwrap_path_parent(parent);
        }

        self.store.exec(|store| {
            for dir in parents.iter().rev() {
                let result = store.create_file(
                    dir.to_string_lossy().as_ref(),
                    FileType::Dir,
                    mode,
                    Some(mtime),
                );

                match result {
                    Ok(_) => {}
                    Err(err) if err.kind() == &crate::ErrorKind::AlreadyExists => {}
                    Err(err) => return Err(err),
                }
            }

            Ok(())
        })
    }

    /// Create a file or directory if it doesn't already exist and set its metadata.
    ///
    /// This accepts the initial file `mode` and `mtime`. It does not care about the current
    /// [`File::umask`].
    ///
    /// # See also
    ///
    /// - [`File::create_file`] to create a regular file with default permissions.
    /// - [`File::create_dir`] to create a directory with default permissions.
    /// - [`File::create_dir_all`] to create a directory and all its parent directories.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::AlreadyExists`]: This file already exists in the archive.
    /// - [`ErrorKind::NotFound`]: This file's parent directory does not exist.
    /// - [`ErrorKind::NotADirectory`]: The file's parent is not a directory.
    ///
    /// [`ErrorKind::AlreadyExists`]: crate::ErrorKind::AlreadyExists
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::NotADirectory`]: crate::ErrorKind::NotADirectory
    pub fn create_with(
        &mut self,
        kind: FileType,
        mode: FileMode,
        mtime: Option<SystemTime>,
    ) -> crate::Result<()> {
        self.validate_can_be_created()?;

        self.store.create_file(&self.path, kind, mode, mtime)
    }

    /// Delete the file from the archive.
    ///
    /// This does not consume its receiver and does not invalidate the file handle; you can still
    /// use this same [`File`] object to create the file again.
    ///
    /// If this file is a directory, this recursively deletes all descendants.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    ///
    /// # Examples
    /// ```
    /// # use sqlarfs::{Connection, FileMode};
    /// # let mut connection = Connection::open_in_memory()?;
    /// # let mut tx = connection.transaction()?;
    /// # let archive = tx.archive_mut();
    /// let mut file = archive.open("file")?;
    ///
    /// file.create_file()?;
    /// assert!(file.exists()?);
    ///
    /// file.delete()?;
    /// assert!(!file.exists()?);
    /// # sqlarfs::Result::Ok(())
    /// ```
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
    /// The mtime in a SQLite archive only has a precision of 1 second, so this rounds down to the
    /// nearest whole second.
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
    /// - [`ErrorKind::IsADirectory`]: The file is a directory.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::IsADirectory`]: crate::ErrorKind::IsADirectory
    pub fn is_empty(&self) -> crate::Result<bool> {
        self.validate_is_readable()?;

        Ok(self.metadata()?.size == 0)
    }

    /// Whether the contents of this file are compressed.
    ///
    /// Even if compression is enabled via [`File::set_compression`], a file may not be compressed
    /// if it's incompressible or if compressing it would *increase* its size.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    /// - [`ErrorKind::IsADirectory`]: The file is a directory.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::IsADirectory`]: crate::ErrorKind::IsADirectory
    pub fn is_compressed(&self) -> crate::Result<bool> {
        self.validate_is_readable()?;

        Ok(self.store.blob_size(&self.path)?.is_compressed())
    }

    /// Truncate the file to zero bytes.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: This file does not exist.
    /// - [`ErrorKind::IsADirectory`]: The file is a directory.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::IsADirectory`]: crate::ErrorKind::IsADirectory
    pub fn truncate(&mut self) -> crate::Result<()> {
        self.validate_is_writable()?;

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
    /// - [`ErrorKind::IsADirectory`]: The file is a directory.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::CompressionNotSupported`]: crate::ErrorKind::CompressionNotSupported
    /// [`ErrorKind::IsADirectory`]: crate::ErrorKind::IsADirectory
    pub fn reader(&mut self) -> crate::Result<FileReader> {
        self.validate_is_readable()?;

        FileReader::new(self.store.open_blob(&self.path, true)?)
    }

    fn write_stream<R>(&mut self, reader: &mut R, size_hint: Option<u64>) -> crate::Result<()>
    where
        R: ?Sized + Read,
    {
        self.validate_is_writable()?;

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
    /// - [`ErrorKind::IsADirectory`]: The file is a directory.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::IsADirectory`]: crate::ErrorKind::IsADirectory
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
    /// - [`ErrorKind::IsADirectory`]: The file is a directory.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::IsADirectory`]: crate::ErrorKind::IsADirectory
    pub fn write_bytes(&mut self, bytes: &[u8]) -> crate::Result<()> {
        self.validate_is_writable()?;

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
    /// - [`ErrorKind::IsADirectory`]: The file is a directory.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::IsADirectory`]: crate::ErrorKind::IsADirectory
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
    /// - [`ErrorKind::IsADirectory`]: The file is a directory.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::IsADirectory`]: crate::ErrorKind::IsADirectory
    pub fn write_file(&mut self, file: &mut fs::File) -> crate::Result<()> {
        // We know the size of the file, which enables some optimizations.
        let metadata = file.metadata()?;
        self.write_stream(file, Some(metadata.len()))
    }

    /// The current compression method used when writing to the file.
    pub fn compression(&self) -> Compression {
        self.compression
    }

    /// Set the compression method used when writing to the file.
    pub fn set_compression(&mut self, method: Compression) {
        self.compression = method;
    }

    /// The current umask for newly created files and directories.
    ///
    /// Files inherit their initial umask from [`Archive::umask`].
    ///
    /// See [`Archive::umask`].
    ///
    /// [`Archive::umask`]: crate::Archive::umask
    pub fn umask(&self) -> FileMode {
        self.umask
    }

    /// Set the umask for newly created files and directories.
    ///
    /// This sets the umask for the current file, but does not affect the  [`Archive::umask`].
    ///
    /// See [`Archive::set_umask`].
    ///
    /// [`Archive::umask`]: crate::Archive::umask
    /// [`Archive::set_umask`]: crate::Archive::set_umask
    ///
    /// # Examples
    ///
    /// ```
    /// # use sqlarfs::{Connection, FileMode};
    /// # let mut connection = Connection::open_in_memory()?;
    /// # let mut tx = connection.transaction()?;
    /// # let archive = tx.archive_mut();
    /// let mut file = archive.open("path/to/file")?;
    /// file.set_umask(FileMode::OTHER_R | FileMode::OTHER_W);
    /// assert_eq!(file.umask(), FileMode::OTHER_R | FileMode::OTHER_W);
    /// # sqlarfs::Result::Ok(())
    /// ```
    pub fn set_umask(&mut self, mode: FileMode) {
        self.umask = mode;
    }
}
