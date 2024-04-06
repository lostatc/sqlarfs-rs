use std::path::Path;

use crate::FileMode;

use super::file::File;
use super::list::{ListEntries, ListOptions};
use super::store::Store;

/// A SQLite archive.
///
/// A SQLite archive is a SQLite database with a table named `sqlar` that conforms to a specific
/// schema. A SQLite archive may contain other tables, and this library will ignore them.
///
/// All file paths in a SQLite archive are relative paths.
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
    pub fn open<'a, P: AsRef<Path>>(&'a mut self, path: P) -> crate::Result<File<'conn, 'a>> {
        // Opening a file must take a mutable receiver to ensure that the user can't get lwo
        // handles to the same file. Otherwise they could do things like open the blob twice or
        // edit the row while the blob is open.
        File::new(path.as_ref(), &mut self.store, self.umask)
    }

    /// Return an iterator over the files in this archive.
    pub fn list(&mut self) -> crate::Result<ListEntries> {
        self.store.list_files(&ListOptions::new())
    }

    /// Return an iterator over the files in this archive.
    ///
    /// This accepts a [`ListOptions`] to sort and filter the results.
    pub fn list_with(&mut self, opts: &ListOptions) -> crate::Result<ListEntries> {
        self.store.list_files(opts)
    }

    /// The current umask for newly created files and directories.
    pub fn umask(&self) -> FileMode {
        self.umask
    }

    /// Set the umask for newly created files and directories.
    ///
    /// This specifies the mode bits that will *not* be set, assuming the default mode for regular
    /// files is `0o666` and the default mode for directories is `0o777`.
    pub fn set_umask(&mut self, mode: FileMode) {
        self.umask = mode;
    }
}
