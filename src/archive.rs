use std::path::Path;

use super::file::File;
use super::transaction::{Transaction, TransactionBehavior};

/// A SQLite archive file.
#[derive(Debug)]
pub struct Archive {
    conn: rusqlite::Connection,
}

impl Archive {
    // TODO: Make private.
    pub fn new(conn: rusqlite::Connection) -> Self {
        Self { conn }
    }

    pub fn transaction(&mut self) -> crate::Result<Transaction> {
        Ok(Transaction::new(self, self.conn.unchecked_transaction()?))
    }

    pub fn transaction_with(
        &mut self,
        behavior: TransactionBehavior,
    ) -> crate::Result<Transaction> {
        Ok(Transaction::new(
            self,
            rusqlite::Transaction::new_unchecked(
                &self.conn,
                match behavior {
                    TransactionBehavior::Deferred => rusqlite::TransactionBehavior::Deferred,
                    TransactionBehavior::Immediate => rusqlite::TransactionBehavior::Immediate,
                    TransactionBehavior::Exclusive => rusqlite::TransactionBehavior::Exclusive,
                },
            )?,
        ))
    }

    /// Create a handle to the file at the given `path`.
    ///
    /// This doesn't guarantee that the file actually exists in the database; it only returns a
    /// handle to a file that may or may not exist.
    ///
    /// See [`File::exists`] to check if the file actually exists in the database.
    pub fn open<P: AsRef<Path>>(&self, path: P) -> File<'_> {
        File::new(path.as_ref().to_path_buf(), &self.conn)
    }
}
