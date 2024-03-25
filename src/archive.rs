use std::path::Path;

use super::file::File;

/// A SQLite archive file.
#[derive(Debug)]
pub struct Archive<'a> {
    conn: &'a rusqlite::Connection,
}

impl<'a> Archive<'a> {
    pub(super) fn new(conn: &'a rusqlite::Connection) -> Self {
        Self { conn }
    }

    /// Create a handle to the file at the given `path`.
    ///
    /// This doesn't guarantee that the file actually exists in the database; it only returns a
    /// handle to a file that may or may not exist.
    ///
    /// See [`File::exists`] to check if the file actually exists in the database.
    pub fn open<P: AsRef<Path>>(&self, path: P) -> File<'_> {
        File::new(path.as_ref().to_path_buf(), self.conn)
    }
}
