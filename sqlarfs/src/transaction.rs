use std::path::Path;

use super::archive::Archive;

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
///
/// You can open a connection to a SQLite archive using one of these methods:
///
/// - [`Connection::open`]
/// - [`Connection::create`]
/// - [`Connection::create_new`]
/// - [`Connection::open_readonly`]
/// - [`Connection::open_in_memory`]
#[derive(Debug)]
pub struct Connection {
    conn: rusqlite::Connection,
}

impl Connection {
    pub(super) fn new(conn: rusqlite::Connection) -> Self {
        Self { conn }
    }

    /// Open a connection to the SQLite archive at `path`.
    ///
    /// This does not create a new SQLite archive if one does not already exist.
    ///
    /// # Errors
    ///
    /// - [`CannotOpen`]: The database could not be opened because it does not exist.
    /// - [`NotADatabase`]: The file at `path` is not a SQLite database.
    ///
    /// [`CannotOpen`]: crate::Error::CannotOpen
    /// [`NotADatabase`]: crate::Error::NotADatabase
    pub fn open<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        use rusqlite::OpenFlags;

        // SQLITE_OPEN_NO_MUTEX is the default in rusqlite. Its docs explain why.
        let flags = OpenFlags::SQLITE_OPEN_NO_MUTEX | OpenFlags::SQLITE_OPEN_READ_WRITE;

        let mut conn = Connection::new(rusqlite::Connection::open_with_flags(path, flags)?);

        conn.exec(|archive| archive.init(false))?;

        Ok(conn)
    }

    /// Create or open the SQLite archive at `path`.
    ///
    /// This creates the SQLite archive if it does not already exist.
    ///
    /// # Errors
    ///
    /// - [`NotADatabase`]: The file at `path` exists but is not a SQLite database.
    ///
    /// [`NotADatabase`]: crate::Error::NotADatabase
    pub fn create<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        use rusqlite::OpenFlags;

        // SQLITE_OPEN_NO_MUTEX is the default in rusqlite. Its docs explain why.
        let flags = OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE;

        let mut conn = Connection::new(rusqlite::Connection::open_with_flags(path, flags)?);

        conn.exec(|archive| archive.init(false))?;

        Ok(conn)
    }

    /// Create a new SQLite archive at `path`.
    ///
    /// This fails if the SQLite archive does not already exist.
    ///
    /// # Errors
    ///
    /// - [`SqlarAlreadyExists`]: A SQLite archive already exists at `path`.
    ///
    /// [`SqlarAlreadyExists`]: crate::Error::SqlarAlreadyExists
    pub fn create_new<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        use rusqlite::OpenFlags;

        // SQLITE_OPEN_NO_MUTEX is the default in rusqlite. Its docs explain why.
        let flags = OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE;

        let mut conn = Connection::new(rusqlite::Connection::open_with_flags(path, flags)?);

        conn.exec(|archive| archive.init(true))?;

        Ok(conn)
    }

    /// Open a read-only connection to the SQLite archive at `path`.
    ///
    /// This does not create a new SQLite archive if one does not already exist.
    ///
    /// # Errors
    ///
    /// - [`CannotOpen`]: The database could not be opened because it does not exist.
    /// - [`NotADatabase`]: The file at `path` is not a SQLite database.
    ///
    /// [`CannotOpen`]: crate::Error::CannotOpen
    /// [`NotADatabase`]: crate::Error::NotADatabase
    pub fn open_readonly<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        use rusqlite::OpenFlags;

        // SQLITE_OPEN_NO_MUTEX is the default in rusqlite. Its docs explain why.
        let flags = OpenFlags::SQLITE_OPEN_NO_MUTEX | OpenFlags::SQLITE_OPEN_READ_ONLY;

        let mut conn = Connection::new(rusqlite::Connection::open_with_flags(path, flags)?);

        conn.exec(|archive| archive.init(false))?;

        Ok(conn)
    }

    /// Create a new in-memory SQLite archive.
    pub fn open_in_memory() -> crate::Result<Self> {
        let mut conn = Self::new(rusqlite::Connection::open_in_memory()?);

        conn.exec(|archive| archive.init(true))?;

        Ok(conn)
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
///
/// # Examples
///
/// ```
/// # use sqlarfs::Connection;
/// let mut connection = Connection::open_in_memory()?;
/// let mut tx = connection.transaction()?;
/// let archive = tx.archive_mut();
///
/// let mut file = archive.open("file")?;
/// file.create_file()?;
///
/// tx.commit()?;
/// # sqlarfs::Result::Ok(())
///
/// ```
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
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn archive(&self) -> &Archive<'conn> {
        &self.archive
    }

    /// Get a mutable reference to the [`Archive`] holding this transaction.
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn archive_mut(&mut self) -> &mut Archive<'conn> {
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
