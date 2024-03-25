use std::path::Path;

use super::db::Store;
use super::file::File;

/// A SQLite archive file.
#[derive(Debug)]
pub struct Archive<'a> {
    store: Store<'a>,
}

impl<'a> Archive<'a> {
    pub(super) fn new(store: Store<'a>) -> Self {
        Self { store }
    }

    /// Create a handle to the file at the given `path`.
    ///
    /// This doesn't guarantee that the file actually exists in the database; it only returns a
    /// handle to a file that may or may not exist.
    ///
    /// See [`File::exists`] to check if the file actually exists in the database.
    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> File<'_> {
        // Opening a file must take a mutable receiver to ensure that the user can't get two
        // handles to the same file. Otherwise they could do things like open the blob twice or
        // edit the row while the blob is open.
        File::new(path.as_ref().to_path_buf(), &self.store)
    }
}
