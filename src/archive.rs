use std::path::Path;

use crate::File;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TransactionBehavior {
    Deferred,
    Immediate,
    Exclusive,
}

#[derive(Debug)]
pub struct Transaction<'a> {
    archive: &'a Archive,
    tx: rusqlite::Transaction<'a>,
}

impl<'a> Transaction<'a> {
    pub fn exec<T, E, F>(self, f: F) -> Result<T, E>
    where
        F: FnOnce(&Archive) -> Result<T, E>,
        E: From<crate::Error>,
    {
        let result = f(self.archive)?;

        self.tx.commit().map_err(crate::Error::from)?;

        Ok(result)
    }

    pub fn archive(&self) -> &Archive {
        self.archive
    }

    pub fn rollback(self) -> crate::Result<()> {
        Ok(self.tx.rollback()?)
    }

    pub fn commit(self) -> crate::Result<()> {
        Ok(self.tx.commit()?)
    }
}

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
        Ok(Transaction {
            archive: self,
            tx: self.conn.unchecked_transaction()?,
        })
    }

    pub fn transaction_with(
        &mut self,
        behavior: TransactionBehavior,
    ) -> crate::Result<Transaction> {
        Ok(Transaction {
            archive: self,
            tx: rusqlite::Transaction::new_unchecked(
                &self.conn,
                match behavior {
                    TransactionBehavior::Deferred => rusqlite::TransactionBehavior::Deferred,
                    TransactionBehavior::Immediate => rusqlite::TransactionBehavior::Immediate,
                    TransactionBehavior::Exclusive => rusqlite::TransactionBehavior::Exclusive,
                },
            )?,
        })
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
