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
/// use this connection to begin a transaction.
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
    pub fn open<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        OpenOptions::new().open(path)
    }

    /// Open a SQLite connection to an in-memory database.
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

#[cfg(test)]
mod tests {
    use xpct::{be_err, be_ok, expect};

    use super::*;

    fn test_transaction_commits_successfully(
        conn: &mut Connection,
        behavior: TransactionBehavior,
    ) -> crate::Result<()> {
        let mut tx = conn.transaction_with(behavior)?;

        tx.archive_mut().init()?;

        tx.commit()?;

        conn.exec(|archive| {
            expect!(archive.open("file").create(None)).to(be_ok());

            Ok(())
        })
    }

    #[test]
    fn transaction_with_deferred_and_commit() -> crate::Result<()> {
        let mut conn = Connection::open_in_memory()?;

        test_transaction_commits_successfully(&mut conn, TransactionBehavior::Deferred)
    }

    #[test]
    fn transaction_with_immediate_and_commit() -> crate::Result<()> {
        let mut conn = Connection::open_in_memory()?;

        test_transaction_commits_successfully(&mut conn, TransactionBehavior::Immediate)
    }

    #[test]
    fn transaction_with_exclusive_and_commit() -> crate::Result<()> {
        let mut conn = Connection::open_in_memory()?;

        test_transaction_commits_successfully(&mut conn, TransactionBehavior::Exclusive)
    }

    fn test_transaction_rolls_back_successfully(
        conn: &mut Connection,
        behavior: TransactionBehavior,
    ) -> crate::Result<()> {
        let mut tx = conn.transaction_with(behavior)?;

        tx.archive_mut().init()?;

        tx.rollback()?;

        conn.exec(|archive| {
            expect!(archive.open("file").create(None)).to(be_err());

            Ok(())
        })
    }

    #[test]
    fn transaction_with_deferred_and_rollback() -> crate::Result<()> {
        let mut conn = Connection::open_in_memory()?;

        test_transaction_rolls_back_successfully(&mut conn, TransactionBehavior::Deferred)
    }

    #[test]
    fn transaction_with_immediate_and_rollback() -> crate::Result<()> {
        let mut conn = Connection::open_in_memory()?;

        test_transaction_rolls_back_successfully(&mut conn, TransactionBehavior::Immediate)
    }

    #[test]
    fn transaction_with_exclusive_and_rollback() -> crate::Result<()> {
        let mut conn = Connection::open_in_memory()?;

        test_transaction_rolls_back_successfully(&mut conn, TransactionBehavior::Exclusive)
    }

    fn test_exec_commits_successfully(
        conn: &mut Connection,
        behavior: TransactionBehavior,
    ) -> crate::Result<()> {
        conn.exec_with(behavior, |archive| archive.init())?;

        conn.exec(|archive| {
            expect!(archive.open("file").create(None)).to(be_ok());

            Ok(())
        })
    }

    #[test]
    fn exec_with_deferred_and_commit() -> crate::Result<()> {
        let mut conn = Connection::open_in_memory()?;

        test_exec_commits_successfully(&mut conn, TransactionBehavior::Deferred)
    }

    #[test]
    fn exec_with_immediate_and_commit() -> crate::Result<()> {
        let mut conn = Connection::open_in_memory()?;

        test_exec_commits_successfully(&mut conn, TransactionBehavior::Immediate)
    }

    #[test]
    fn exec_with_exclusive_and_commit() -> crate::Result<()> {
        let mut conn = Connection::open_in_memory()?;

        test_exec_commits_successfully(&mut conn, TransactionBehavior::Exclusive)
    }
}
