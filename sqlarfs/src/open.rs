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
    init_new: bool,
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
            init_new: false,
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
    /// This is mutually exclusive with [`OpenOptions::init_new`].
    ///
    /// The default is `true`.
    pub fn init(&mut self, init: bool) -> &mut Self {
        self.init = init;

        self
    }

    /// Set whether to create the `sqlar` table in the database and fail if it already exists.
    ///
    /// This sets [`OpenOptions::init`] to `false`. If it's overridden and set to `true`, then
    /// [`OpenOptions::open`] will return an error.
    ///
    /// See [`OpenOptions::init`] for more information.
    ///
    /// The default is `false`.
    pub fn init_new(&mut self, init_new: bool) -> &mut Self {
        self.init_new = init_new;
        self.init = false;

        self
    }

    /// Set whether the database should be read-only.
    ///
    /// This sets [`OpenOptions::create`], [`OpenOptions::init`], and [`OpenOptions::init_new`] to
    /// `false`. If any are overridden and set to `true`, then [`OpenOptions::open`] will return an
    /// error.
    ///
    /// The default is `false`.
    pub fn read_only(&mut self, read_only: bool) -> &mut Self {
        self.read_only = read_only;
        self.create = false;
        self.init = false;
        self.init_new = false;

        self
    }

    /// Open a new database [`Connection`] at the given `path`.
    pub fn open<P: AsRef<Path>>(&mut self, path: P) -> crate::Result<Connection> {
        use rusqlite::OpenFlags;

        if self.init && self.init_new {
            return Err(crate::Error::msg(
                crate::ErrorKind::InvalidArgs,
                "`OpenOptions::init` and `OpenOptions::init_new` are mutually exclusive.",
            ));
        }

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

        let mut conn = Connection::new(rusqlite::Connection::open_with_flags(path, flags)?);

        if self.init | self.init_new {
            conn.exec(|archive| archive.init(self.init_new))?;
        }

        Ok(conn)
    }
}
