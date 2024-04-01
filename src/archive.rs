use std::path::Path;

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
}

impl<'conn> Archive<'conn> {
    pub(super) fn new(tx: rusqlite::Transaction<'conn>) -> Self {
        Self {
            store: Store::new(tx),
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
    /// # Errors
    ///
    /// - [`ErrorKind::PathIsAbsolute`]: The given `path` is an absolute path.
    ///
    /// [`ErrorKind::PathIsAbsolute`]: crate::ErrorKind::PathIsAbsolute
    pub fn open<'a, P: AsRef<Path>>(&'a mut self, path: P) -> crate::Result<File<'conn, 'a>> {
        // Opening a file must take a mutable receiver to ensure that the user can't get two
        // handles to the same file. Otherwise they could do things like open the blob twice or
        // edit the row while the blob is open.
        File::new(path.as_ref().to_path_buf(), &mut self.store)
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
}
