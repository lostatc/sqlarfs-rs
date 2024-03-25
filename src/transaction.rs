use std::path::Path;

use super::archive::Archive;
use super::open::OpenOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransactionBehavior {
    Deferred,
    Immediate,
    Exclusive,
}

impl TransactionBehavior {
    fn inner(self) -> rusqlite::TransactionBehavior {
        match self {
            TransactionBehavior::Deferred => rusqlite::TransactionBehavior::Deferred,
            TransactionBehavior::Immediate => rusqlite::TransactionBehavior::Immediate,
            TransactionBehavior::Exclusive => rusqlite::TransactionBehavior::Exclusive,
        }
    }
}

/// A connection to a SQLite database holding a sqlar archive.
#[derive(Debug)]
pub struct Connection {
    conn: rusqlite::Connection,
}

impl Connection {
    pub(super) fn new(conn: rusqlite::Connection) -> Self {
        Self { conn }
    }

    /// Create a new builder for opening a [`Connection`].
    pub fn builder() -> OpenOptions {
        OpenOptions::new()
    }

    pub fn open<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        OpenOptions::new().open(path)
    }

    pub fn open_in_memory() -> crate::Result<Self> {
        OpenOptions::new().open_in_memory()
    }

    fn archive(&self) -> Archive {
        Archive::new(&self.conn)
    }

    // Opening a transaction must take a mutable receiver to ensure that the user can't open more
    // than one transaction at a time.

    pub fn transaction(&mut self) -> crate::Result<Transaction> {
        Ok(Transaction::new(
            self.archive(),
            self.conn.unchecked_transaction()?,
        ))
    }

    pub fn transaction_with(
        &mut self,
        behavior: TransactionBehavior,
    ) -> crate::Result<Transaction> {
        Ok(Transaction::new(
            self.archive(),
            rusqlite::Transaction::new_unchecked(&self.conn, behavior.inner())?,
        ))
    }

    pub fn exec<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&Archive) -> Result<T, E>,
        E: From<crate::Error>,
    {
        self.transaction()?.exec(f)
    }

    pub fn exec_with<T, E, F>(&mut self, behavior: TransactionBehavior, f: F) -> Result<T, E>
    where
        F: FnOnce(&Archive) -> Result<T, E>,
        E: From<crate::Error>,
    {
        self.transaction_with(behavior)?.exec(f)
    }
}

/// An open transaction on an [`Archive`].
///
/// If a `Transaction` is dropped without committing, the transaction is rolled back.
#[derive(Debug)]
pub struct Transaction<'a> {
    archive: Archive<'a>,
    tx: rusqlite::Transaction<'a>,
}

impl<'a> Transaction<'a> {
    pub(super) fn new(archive: Archive<'a>, tx: rusqlite::Transaction<'a>) -> Self {
        Self { archive, tx }
    }

    /// Execute this transaction.
    ///
    /// This calls the given function, passing the [`Archive`] holding this transaction. If the
    /// function returns `Ok`, this transaction is committed. If the function returns `Err`, this
    /// transaction is rolled back.
    pub fn exec<T, E, F>(self, f: F) -> Result<T, E>
    where
        F: FnOnce(&Archive) -> Result<T, E>,
        E: From<crate::Error>,
    {
        let result = f(&self.archive)?;

        self.tx.commit().map_err(crate::Error::from)?;

        Ok(result)
    }

    /// Get a reference to the [`Archive`] holding this transaction.
    pub fn archive(&self) -> &Archive {
        &self.archive
    }

    /// Roll back this transaction.
    pub fn rollback(self) -> crate::Result<()> {
        Ok(self.tx.rollback()?)
    }

    /// Commit this transaction.
    pub fn commit(self) -> crate::Result<()> {
        Ok(self.tx.commit()?)
    }
}
