use std::path::Path;

use super::transaction::Connection;

/// A builder for opening a database [`Connection`].
///
/// You can create a new builder with [`Connection::builder`].
///
/// You can also use the [`Connection::open`] convenience method. To open an in-memory database,
/// use [`Connection::open_in_memory`].
#[derive(Debug, Clone)]
pub struct OpenOptions {
    create: Option<bool>,
    create_new: Option<bool>,
    read_only: Option<bool>,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenOptions {
    /// Create a new [`OpenOptions`] builder.
    pub fn new() -> Self {
        Self {
            create: None,
            create_new: None,
            read_only: None,
        }
    }

    /// Set whether to create the SQLite archive if it doesn't already exist.
    ///
    /// This is mutually exclusive with [`OpenOptions::create_new`].
    ///
    /// The default is `false`.
    pub fn create(&mut self, create: bool) -> &mut Self {
        self.create = Some(create);

        self
    }

    /// Set whether to create the SQLite archive, failing if it already exists.
    ///
    /// This is mutually exclusive with [`OpenOptions::create`].
    ///
    /// The default is `false`.
    pub fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.create_new = Some(create_new);

        self
    }

    /// Set whether the database should be read-only.
    ///
    /// You cannot create a new database in read-only mode. This is mutually exclusive with
    /// [`OpenOptions::create`] and [`OpenOptions::create_new`].
    ///
    /// The default is `false`.
    pub fn read_only(&mut self, read_only: bool) -> &mut Self {
        self.read_only = Some(read_only);

        self
    }

    fn validate_options(&self) -> crate::Result<()> {
        if self.create == Some(true) && self.create_new == Some(true) {
            return Err(crate::Error::InvalidArgs {
                reason: String::from(
                    "`OpenOptions::create` and `OpenOptions::create_new` are mutually exclusive.",
                ),
            });
        }

        if self.read_only == Some(true) && self.create == Some(true) {
            return Err(crate::Error::InvalidArgs {
                reason: String::from(
                    "`OpenOptions::read_only` and `OpenOptions::create` are mutually exclusive.",
                ),
            });
        }

        if self.read_only == Some(true) && self.create_new == Some(true) {
            return Err(crate::Error::InvalidArgs {
                reason: String::from(
                    "`OpenOptions::read_only` and `OpenOptions::create_new` are mutually exclusive.",
                ),
            });
        }

        Ok(())
    }

    /// Open a new database [`Connection`] at the given `path`.
    ///
    /// # Errors
    ///
    /// - [`CannotOpen`]: The database could not be opened for some reason, such as because it does
    /// not exist.
    /// - [`NotADatabase`]: The file at `path` is not a SQLite database.
    /// - [`SqlarAlreadyExists`]: [`OpenOptions::create_new`] was `true`, but the `sqlar` table
    /// already exists.
    ///
    /// [`CannotOpen`]: crate::Error::CannotOpen
    /// [`NotADatabase`]: crate::Error::NotADatabase
    /// [`SqlarAlreadyExists`]: crate::Error::SqlarAlreadyExists
    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> crate::Result<Connection> {
        use rusqlite::OpenFlags;

        self.validate_options()?;

        // SQLITE_OPEN_NO_MUTEX is the default in rusqlite. Its docs explain why.
        let mut flags = OpenFlags::SQLITE_OPEN_NO_MUTEX;

        if self.read_only.unwrap_or(false) {
            flags |= OpenFlags::SQLITE_OPEN_READ_ONLY;
        } else {
            flags |= OpenFlags::SQLITE_OPEN_READ_WRITE;
        }

        if self.create.unwrap_or(false) || self.create_new.unwrap_or(false) {
            flags |= OpenFlags::SQLITE_OPEN_CREATE;
        }

        let mut conn = Connection::new(rusqlite::Connection::open_with_flags(path, flags)?);

        conn.exec(|archive| archive.init(self.create_new.unwrap_or(false)))?;

        Ok(conn)
    }
}
