use std::path::Path;

use crate::FileMode;

use super::file::File;
use super::list::{ListEntries, ListOptions};
use super::store::Store;
use super::tree::{archive_tree, ArchiveOptions};

/// A SQLite archive.
///
/// This is the main type for reading and writing to the archive. You can only access an `Archive`
/// within the context of a transaction, which you'll typically use [`Connection::exec`] for.
///
/// A SQLite archive is a SQLite database with a table named `sqlar` that conforms to a specific
/// schema. A SQLite archive may contain other tables, and this library will ignore them.
///
/// All file paths in a SQLite archive are relative paths.
///
/// [`Connection::exec`]: crate::Connection::exec
#[derive(Debug)]
pub struct Archive<'conn> {
    store: Store<'conn>,
    umask: FileMode,
}

impl<'conn> Archive<'conn> {
    pub(super) fn new(tx: rusqlite::Transaction<'conn>) -> Self {
        Self {
            store: Store::new(tx),
            umask: FileMode::OTHER_W,
        }
    }

    pub(super) fn into_tx(self) -> rusqlite::Transaction<'conn> {
        self.store.into_tx()
    }

    pub(super) fn init(&mut self) -> crate::Result<()> {
        self.store.create_table()
    }

    /// Create a handle to the file at the given `path`.
    ///
    /// This doesn't guarantee that the file actually exists in the archive; it only returns a
    /// handle to a file that may or may not exist.
    ///
    /// See [`File::exists`] to check if the file actually exists in the archive.
    ///
    /// All file paths in a SQLite archive are relative paths; this method returns an error if the
    /// given `path` is an absolute path.
    ///
    /// All file paths in a SQLite archive are encoded using the database encoding; this method
    /// returns an error if the given `path` is not valid Unicode.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::InvalidArgs`]: The given `path` is an absolute path.
    /// - [`ErrorKind::InvalidArgs`]: The given `path` is not valid Unicode.
    /// - [`ErrorKind::InvalidArgs`]: The given `path` is empty.
    ///
    /// [`ErrorKind::InvalidArgs`]: crate::ErrorKind::InvalidArgs
    pub fn open<'ar, P: AsRef<Path>>(&'ar mut self, path: P) -> crate::Result<File<'conn, 'ar>> {
        // Opening a file must take a mutable receiver to ensure that the user can't get lwo
        // handles to the same file. Otherwise they could do things like open the blob twice or
        // edit the row while the blob is open.
        File::new(path.as_ref(), &mut self.store, self.umask)
    }

    /// Return an iterator over the files in this archive.
    ///
    /// This is the same as [`Archive::list_with`], but using the default options.
    pub fn list(&mut self) -> crate::Result<ListEntries> {
        self.store.list_files(&ListOptions::new())
    }

    /// Return an iterator over the files in this archive.
    ///
    /// This accepts a [`ListOptions`] to sort and filter the results.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::InvalidArgs`]: Mutually exclusive options were specified together in
    /// [`ListOptions`].
    ///
    /// # Examples
    ///
    /// List the regular files that are descendants of `parent/dir` in descending order by size.
    ///
    /// ```
    /// # use sqlarfs::{ListOptions, Connection};
    /// # let mut connection = Connection::open_in_memory()?;
    /// # let mut tx = connection.transaction()?;
    /// # let mut archive = tx.archive_mut();
    /// let opts = ListOptions::new().by_size().desc().descendants_of("parent/dir");
    ///
    /// for result in archive.list_with(&opts)? {
    ///     let entry = result?;
    ///     println!("{}: {}", entry.path().to_string_lossy(), entry.metadata().size);
    /// }
    /// # sqlarfs::Result::Ok(())
    /// ```
    ///
    /// [`ErrorKind::InvalidArgs`]: crate::ErrorKind::InvalidArgs
    pub fn list_with(&mut self, opts: &ListOptions) -> crate::Result<ListEntries> {
        if opts.is_invalid {
            return Err(crate::Error::msg(
                crate::ErrorKind::InvalidArgs,
                "Mutually exclusive options where used together in `ListOptions`.",
            ));
        }

        self.store.list_files(opts)
    }

    /// Copy the filesystem directory tree at `from` into the archive at `to`.
    ///
    /// This is the same as [`Archive::archive_with`], but using the default options.
    pub fn archive<P: AsRef<Path>, Q: AsRef<Path>>(&mut self, from: P, to: Q) -> crate::Result<()> {
        self.archive_with(from, to, &Default::default())
    }

    /// Copy the filesystem directory tree at `from` into the archive at `to`.
    ///
    /// The file at `from` may be either a directory or a regular file.
    ///
    /// # Errors
    ///
    /// - [`ErrorKind::NotFound`]: There is no file or directory at `from`.
    /// - [`ErrorKind::NotFound`]: The parent directory of `to` does not exist.
    /// - [`ErrorKind::NotFound`]: [`ArchiveOptions::children`] was `true` and `to` does not exist.
    /// - [`ErrorKind::NotADirectory`]: [`ArchiveOptions::children`] was `true` and the file at
    /// `to` already exists and is not a directory.
    /// - [`ErrorKind::AlreadyExists`]: One of the files in `from` would overwrite an existing file
    /// in the archive.
    /// - [`ErrorKind::InvalidArgs`]: The given `to` path is an absolute path.
    /// - [`ErrorKind::InvalidArgs`]: The given `to` path is not valid Unicode.
    ///
    /// [`ErrorKind::NotFound`]: crate::ErrorKind::NotFound
    /// [`ErrorKind::NotADirectory`]: crate::ErrorKind::NotADirectory
    /// [`ErrorKind::AlreadyExists`]: crate::ErrorKind::AlreadyExists
    /// [`ErrorKind::InvalidArgs`]: crate::ErrorKind::InvalidArgs
    pub fn archive_with<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        from: P,
        to: Q,
        opts: &ArchiveOptions,
    ) -> crate::Result<()> {
        // On Unix-like systems, we set the file mode based on the mode bits in the archive.
        #[cfg(unix)]
        archive_tree(
            self,
            from.as_ref(),
            to.as_ref(),
            opts,
            &super::mode::UnixModeAdapter,
        )?;

        // On unsupported platforms (currently any non-Unix-like platform), we use the umask.
        #[cfg(not(unix))]
        archive_tree(
            self,
            from.as_ref(),
            to.as_ref(),
            opts,
            &super::mode::UmaskModeAdapter::new(self.umask),
        )?;

        Ok(())
    }

    /// The current umask for newly created files and directories.
    pub fn umask(&self) -> FileMode {
        self.umask
    }

    /// Set the umask for newly created files and directories.
    ///
    /// This specifies the mode bits that will *not* be set, assuming the default mode for regular
    /// files is `0o666` and the default mode for directories is `0o777`.
    ///
    /// The default umask is `FileMode::OTHER_W` (`0o002`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use sqlarfs::{Connection, FileMode};
    /// # let mut connection = Connection::open_in_memory()?;
    /// # let mut tx = connection.transaction()?;
    /// # let archive = tx.archive_mut();
    /// archive.set_umask(FileMode::OTHER_R | FileMode::OTHER_W);
    /// assert_eq!(archive.umask(), FileMode::OTHER_R | FileMode::OTHER_W);
    /// # sqlarfs::Result::Ok(())
    /// ```
    pub fn set_umask(&mut self, mode: FileMode) {
        self.umask = mode;
    }
}
