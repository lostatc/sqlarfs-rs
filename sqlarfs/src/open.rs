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
    create: bool,
    init: bool,
    read_only: bool,
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
            create: true,
            init: true,
            read_only: false,
        }
    }

    /// Set whether to create the database if it doesn't already exist in the filesystem.
    ///
    /// The default is `true`.
    pub fn create(&mut self, create: bool) -> &mut Self {
        self.create = create;

        self
    }

    /// Set whether to create the `sqlar` table if it doesn't already exist in the database.
    ///
    /// A SQLite archive is a SQLite database with a table named `sqlar` that conforms to a
    /// specific schema. That table needs to exist in order to use this database as a SQLite
    /// archive.
    ///
    /// The default is `true`.
    pub fn init(&mut self, init: bool) -> &mut Self {
        self.init = init;

        self
    }

    /// Set whether the database should be read-only.
    ///
    /// This sets both [`OpenOptions::create`] and [`OpenOptions::init`] to `false`. If either is
    /// overridden and set to `true`, then [`OpenOptions::open`] will return an error.
    ///
    /// The default is `false`.
    pub fn read_only(&mut self, read_only: bool) -> &mut Self {
        self.read_only = read_only;
        self.create = false;
        self.init = false;

        self
    }

    /// Open a new database [`Connection`] at the given `path`.
    ///
    /// # Errors
    ///
    /// - [`CannotOpen`]: The database does not exist and [`OpenOptions::create`] was `false`.
    ///
    /// [`CannotOpen`]: crate::ErrorKind::CannotOpen
    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> crate::Result<Connection> {
        use rusqlite::OpenFlags;

        // SQLITE_OPEN_NO_MUTEX is the default in rusqlite. Its docs explain why.
        let mut flags = OpenFlags::SQLITE_OPEN_NO_MUTEX;

        if self.read_only {
            flags |= OpenFlags::SQLITE_OPEN_READ_ONLY;
        } else {
            flags |= OpenFlags::SQLITE_OPEN_READ_WRITE;
        }

        if self.create {
            flags |= OpenFlags::SQLITE_OPEN_CREATE;
        }

        let mut conn = match rusqlite::Connection::open_with_flags(path, flags) {
            Ok(conn) => Connection::new(conn),
            Err(err) => match err.sqlite_error_code() {
                Some(rusqlite::ErrorCode::ApiMisuse) => {
                    return Err(crate::Error::new(crate::ErrorKind::InvalidArgs, err))
                }
                _ => return Err(err.into()),
            },
        };

        if self.init {
            conn.exec(|archive| match archive.init() {
                Err(err) if err.kind() == &crate::ErrorKind::ReadOnly => {
                    Err(crate::Error::new(crate::ErrorKind::InvalidArgs, err))
                }
                result => result,
            })?;
        }

        Ok(conn)
    }
}