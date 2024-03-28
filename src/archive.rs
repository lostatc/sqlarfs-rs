use std::path::Path;

use super::file::File;
use super::store::Store;

/// A SQLite archive file.
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

    /// Create the `sqlar` table in the database if it doesn't already exist.
    ///
    /// You only need to do this once per database. This does not fail if the table already exists.
    pub fn init(&mut self) -> crate::Result<()> {
        self.store.create_table()
    }

    /// Create a handle to the file at the given `path`.
    ///
    /// This doesn't guarantee that the file actually exists in the database; it only returns a
    /// handle to a file that may or may not exist.
    ///
    /// See [`File::exists`] to check if the file actually exists in the database.
    pub fn open<'a, P: AsRef<Path>>(&'a mut self, path: P) -> File<'conn, 'a> {
        // Opening a file must take a mutable receiver to ensure that the user can't get two
        // handles to the same file. Otherwise they could do things like open the blob twice or
        // edit the row while the blob is open.
        File::new(path.as_ref().to_path_buf(), &mut self.store)
    }
}
