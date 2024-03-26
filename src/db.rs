use std::path::Path;

use rusqlite::{blob::Blob, OptionalExtension};

#[derive(Debug)]
pub struct Store<'a> {
    conn: &'a rusqlite::Connection,
}

impl<'a> Store<'a> {
    pub fn new(conn: &'a rusqlite::Connection) -> Self {
        Self { conn }
    }

    pub fn open_blob(&self, path: &Path, read_only: bool) -> crate::Result<Blob> {
        let row = self
            .conn
            .query_row(
                "SELECT rowid FROM sqlar WHERE path = ?1;",
                (path.to_string_lossy(),),
                |row| row.get(0),
            )
            .optional()?;

        match row {
            Some(row_id) => Ok(self.conn.blob_open(
                rusqlite::DatabaseName::Main,
                "sqlar",
                "data",
                row_id,
                read_only,
            )?),
            None => Err(crate::Error::NotFound),
        }
    }
}
