use std::path::Path;

use crate::{ExtractOptions, FileMode};

use super::file::File;
use super::list::{ListEntries, ListOptions};
use super::store::Store;
use super::tree::ArchiveOptions;

#[cfg(all(unix, feature = "fuse"))]
use super::fuse::MountOption;

/// A SQLite archive.
///
/// This is the main type for reading and writing to the archive. You can only access an `Archive`
/// within the context of a transaction, which you'll typically use [`Connection::exec`] for.
///
/// A SQLite archive is a SQLite database with a table named `sqlar` that conforms to a specific
/// schema. A SQLite archive may contain other tables, and this library will ignore them.
///
/// All file paths in a SQLite archive are relative paths; trying to use an absolute path will
/// result in an error.
///
/// All file paths in a SQLite archive are encoded using the database encoding; trying to use a
/// path that is not valid Unicode will result in an error.
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

    pub(super) fn init(&mut self, fail_if_exists: bool) -> crate::Result<()> {
        self.store.create_table(fail_if_exists)
    }

    /// Create a handle to the file at the given `path`.
    ///
    /// This doesn't guarantee that the file actually exists in the archive; it only returns a
    /// handle to a file that may or may not exist.
    ///
    /// See [`File::exists`] to check if the file actually exists in the archive.
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
    /// This returns an error if mutually exclusive options were specified together in
    /// [`ListOptions`].
    ///
    /// # Examples
    ///
    /// List the regular files that are descendants of `parent/dir` in descending order by size.
    ///
    /// ```
    /// # use sqlarfs::{ListOptions, Connection, FileMetadata};
    /// # let mut connection = Connection::open_in_memory()?;
    /// # let mut tx = connection.transaction()?;
    /// # let mut archive = tx.archive_mut();
    /// let opts = ListOptions::new().by_size().desc().descendants_of("parent/dir");
    ///
    /// for result in archive.list_with(&opts)? {
    ///     let entry = result?;
    ///     let path = entry.path();
    ///
    ///     if let FileMetadata::File { size, .. } = entry.metadata() {
    ///         println!("{}: {}", path.to_string_lossy(), size);
    ///     }
    /// }
    /// # sqlarfs::Result::Ok(())
    /// ```
    pub fn list_with(&mut self, opts: &ListOptions) -> crate::Result<ListEntries> {
        if opts.is_invalid {
            return Err(crate::Error::InvalidArgs {
                reason: String::from(
                    "Mutually exclusive options where used together in `ListOptions`.",
                ),
            });
        }

        self.store.list_files(opts)
    }

    /// Copy the filesystem directory tree at `from` into the archive at `to`.
    ///
    /// This is the same as [`Archive::archive_with`], but using the default options.
    pub fn archive<P: AsRef<Path>, Q: AsRef<Path>>(&mut self, from: P, to: Q) -> crate::Result<()> {
        self.archive_with(from, to, &Default::default())
    }

    /// Copy the directory tree at in the filesystem at `from` into the archive at `to`.
    ///
    /// The file at `from` may be either a directory or a regular file.
    ///
    /// # Errors
    ///
    /// - [`FileNotFound`]: There is no file or directory at `from`.
    /// - [`FileNotFound`]: [`ArchiveOptions::children`] was `true` and `to` does not exist.
    /// - [`NoParentDirectory`]: The parent directory of `to` does not exist.
    /// - [`NotADirectory`]: [`ArchiveOptions::children`] was `true` and the file at `from` is not
    /// a directory.
    /// - [`NotADirectory`]: [`ArchiveOptions::children`] was `true` and the file at `to` exists
    /// but is not a directory.
    /// - [`FileAlreadyExists`]: One of the files in `from` would overwrite an existing file in the
    /// archive.
    ///
    /// [`FileNotFound`]: crate::Error::FileNotFound
    /// [`NoParentDirectory`]: crate::Error::NoParentDirectory
    /// [`NotADirectory`]: crate::Error::NotADirectory
    /// [`FileAlreadyExists`]: crate::Error::FileAlreadyExists
    pub fn archive_with<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        from: P,
        to: Q,
        opts: &ArchiveOptions,
    ) -> crate::Result<()> {
        self.archive_tree(
            from.as_ref(),
            to.as_ref(),
            opts,
            #[cfg(unix)]
            &super::mode::UnixModeAdapter,
            #[cfg(windows)]
            &super::mode::WindowsModeAdapter,
        )
    }

    /// Copy the directory tree in the archive at `from` into the filesystem at `to`.
    ///
    /// This is the same as [`Archive::extract_with`], but using the default options.
    pub fn extract<P: AsRef<Path>, Q: AsRef<Path>>(&mut self, from: P, to: Q) -> crate::Result<()> {
        self.extract_with(from, to, &Default::default())
    }

    /// Copy the directory tree in the archive at `from` into the filesystem at `to`.
    ///
    /// The file at `from` may be either a directory or a regular file.
    ///
    /// # Errors
    ///
    /// - [`FileNotFound`]: There is no file or directory in the archive at `from`.
    /// - [`FileNotFound`]: [`ExtractOptions::children`] was `true` and `to` does not exist.
    /// - [`NoParentDirectory`]: The parent directory of `to` does not exist.
    /// - [`NotADirectory`]: [`ExtractOptions::children`] was `true` and the file at `from` is not
    /// a directory.
    /// - [`NotADirectory`]: [`ExtractOptions::children`] was `true` and the file at `to` exists
    /// but is not a directory.
    /// - [`FileAlreadyExists`]: One of the files in `from` would overwrite an existing file in the
    /// filesystem.
    ///
    /// [`FileNotFound`]: crate::Error::FileNotFound
    /// [`NoParentDirectory`]: crate::Error::NoParentDirectory
    /// [`NotADirectory`]: crate::Error::NotADirectory
    /// [`FileAlreadyExists`]: crate::Error::FileAlreadyExists
    pub fn extract_with<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        from: P,
        to: Q,
        opts: &ExtractOptions,
    ) -> crate::Result<()> {
        self.extract_tree(
            from.as_ref(),
            to.as_ref(),
            opts,
            #[cfg(unix)]
            &super::mode::UnixModeAdapter,
            #[cfg(windows)]
            &super::mode::WindowsModeAdapter,
        )
    }

    /// Mount the archive as a FUSE file system.
    ///
    /// This accepts the path of the `root` entry in the repository which will be mounted in the
    /// file system at `mountpoint`. This also accepts an array of mount `options` to pass to
    /// libfuse.
    ///
    /// This method does not return until the file system is unmounted.
    ///
    /// # Errors
    ///
    /// - [`FileNotFound`]: There is no file in the archive at `root`.
    /// - [`NotADirectory`]: The file at `root` is not a directory.
    ///
    /// [`FileNotFound`]: crate::Error::FileNotFound
    /// [`NotADirectory`]: crate::Error::NotADirectory
    #[cfg(all(unix, feature = "fuse"))]
    pub fn mount<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        mountpoint: P,
        root: Q,
        options: &[MountOption],
    ) -> crate::Result<()> {
        use std::collections::HashSet;

        use crate::fuse::{default_mount_opts, FuseAdapter};

        // These need to be deduplicated.
        let all_opts = [default_mount_opts(), options.to_vec()]
            .concat()
            .into_iter()
            .map(|opt| opt.into_fuser())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        let adapter = FuseAdapter::new(self, root.as_ref())?;

        Ok(fuser::mount2(adapter, &mountpoint, &all_opts)?)
    }

    /// The current umask for newly created files and directories.
    pub fn umask(&self) -> FileMode {
        self.umask
    }

    /// Set the umask for newly created files and directories.
    ///
    /// This specifies the mode bits that will *not* be set, assuming the default mode for regular
    /// files is `666` and the default mode for directories is `777`.
    ///
    /// The default umask is `FileMode::OTHER_W` (`002`).
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
