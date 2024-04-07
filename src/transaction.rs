use std::path::Path;

use super::archive::Archive;
use super::open::OpenOptions;

/// The behavior of a SQLite transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransactionBehavior {
    /// DEFERRED means that the transaction does not actually start until the database is first
    /// accessed.
    Deferred,

    /// IMMEDIATE cause the database connection to start a new write immediately, without waiting
    /// for a writes statement.
    Immediate,

    /// EXCLUSIVE prevents other database connections from reading the database while the
    /// transaction is underway.
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

/// A connection to a SQLite database.
///
/// All operations on an [`Archive`] must happen within the context of a [`Transaction`]. You can
/// use this connection to begin a transaction. Typically, you'll use [`Connection::exec`] to
/// execute a closure within a transaction.
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

    /// Open a SQLite connection to the file at `path`.
    ///
    /// You can access more options for how the connection is opened with [`Connection::builder`].
    pub fn open<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        OpenOptions::new().open(path)
    }

    /// Open a SQLite connection to an in-memory database.
    ///
    /// You can access more options for how the connection is opened with [`Connection::builder`].
    pub fn open_in_memory() -> crate::Result<Self> {
        OpenOptions::new().open_in_memory()
    }

    /// Start a new transaction.
    pub fn transaction(&mut self) -> crate::Result<Transaction> {
        Ok(Transaction::new(self.conn.transaction()?))
    }

    /// Start a new transaction with the given [`TransactionBehavior`].
    pub fn transaction_with(
        &mut self,
        behavior: TransactionBehavior,
    ) -> crate::Result<Transaction> {
        Ok(Transaction::new(
            self.conn.transaction_with_behavior(behavior.inner())?,
        ))
    }

    /// Execute the given function within a new transaction.
    ///
    /// See [`Transaction::exec`].
    pub fn exec<T, E, F>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Archive) -> Result<T, E>,
        E: From<crate::Error>,
    {
        self.transaction()?.exec(f)
    }

    /// Execute the given function within a new transaction with the given
    /// [`TransactionBehavior`].,
    ///
    /// See [`Transaction::exec`].
    pub fn exec_with<T, E, F>(&mut self, behavior: TransactionBehavior, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Archive) -> Result<T, E>,
        E: From<crate::Error>,
    {
        self.transaction_with(behavior)?.exec(f)
    }
}

/// An open transaction on an [`Archive`].
///
/// If a `Transaction` is dropped without committing, the transaction is rolled back.
#[derive(Debug)]
pub struct Transaction<'conn> {
    archive: Archive<'conn>,
}

impl<'conn> Transaction<'conn> {
    pub(super) fn new(tx: rusqlite::Transaction<'conn>) -> Self {
        Self {
            archive: Archive::new(tx),
        }
    }

    /// Execute the given function within this transaction.
    ///
    /// This calls the given function, passing the [`Archive`] holding this transaction. If the
    /// function returns `Ok`, this transaction is committed. If the function returns `Err`, this
    /// transaction is rolled back.
    pub fn exec<T, E, F>(mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Archive) -> Result<T, E>,
        E: From<crate::Error>,
    {
        let result = f(&mut self.archive)?;

        self.archive
            .into_tx()
            .commit()
            .map_err(crate::Error::from)?;

        Ok(result)
    }

    /// Get a reference to the [`Archive`] holding this transaction.
    pub fn archive(&self) -> &Archive<'conn> {
        &self.archive
    }

    /// Get a mutable reference to the [`Archive`] holding this transaction.
    pub fn archive_mut<'a>(&'a mut self) -> &'a mut Archive<'conn> {
        &mut self.archive
    }

    /// Roll back this transaction.
    pub fn rollback(self) -> crate::Result<()> {
        Ok(self.archive.into_tx().rollback()?)
    }

    /// Commit this transaction.
    pub fn commit(self) -> crate::Result<()> {
        Ok(self.archive.into_tx().commit()?)
    }
}
