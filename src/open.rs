use std::path::Path;

use super::transaction::Connection;

/// A builder for opening a database [`Connection`].
#[derive(Debug, Default)]
pub struct OpenOptions {
    create: bool,
    read_only: bool,
}

impl OpenOptions {
    /// Create a new [`OpenOptions`] builder.
    pub fn new() -> Self {
        Self {
            create: true,
            read_only: false,
        }
    }

    /// Set whether the database should be created if it doesn't already exist in the filesystem.
    ///
    /// The default is `true`.
    pub fn create(&mut self, create: bool) -> &mut Self {
        self.create = create;

        self
    }

    /// Set whether the database should be read-only.
    ///
    /// The default is `false`.
    pub fn read_only(&mut self, read_only: bool) -> &mut Self {
        self.read_only = read_only;

        self
    }

    /// Open a new database [`Connection`] at the given `path`.
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

        let conn = rusqlite::Connection::open_with_flags(path, flags)?;

        Ok(Connection::new(conn))
    }

    /// Open a new in-memory database [`Connection`].
    pub fn open_in_memory(&mut self) -> crate::Result<Connection> {
        use rusqlite::OpenFlags;

        // SQLITE_OPEN_NO_MUTEX is the default in rusqlite. Its docs explain why.
        let mut flags = OpenFlags::SQLITE_OPEN_MEMORY | OpenFlags::SQLITE_OPEN_NO_MUTEX;

        if self.read_only {
            flags |= OpenFlags::SQLITE_OPEN_READ_ONLY;
        } else {
            flags |= OpenFlags::SQLITE_OPEN_READ_WRITE;
        }

        if self.create {
            flags |= OpenFlags::SQLITE_OPEN_CREATE;
        }

        let conn = rusqlite::Connection::open_in_memory_with_flags(flags)?;

        Ok(Connection::new(conn))
    }
}
