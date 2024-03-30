use std::path::Path;
use std::time::{self, SystemTime};

use rusqlite::blob::Blob;
use rusqlite::{OptionalExtension, Savepoint};

use super::metadata::FileMode;
use super::util::u64_from_usize;

#[derive(Debug)]
enum InnerTransaction<'conn> {
    Transaction(rusqlite::Transaction<'conn>),
    Savepoint(rusqlite::Savepoint<'conn>),
}

pub struct FileBlob<'conn> {
    blob: Blob<'conn>,
    original_size: u64,
}

impl<'conn> FileBlob<'conn> {
    pub fn is_compressed(&self) -> bool {
        u64_from_usize(self.blob.len()) != self.original_size
    }

    pub fn into_blob(self) -> Blob<'conn> {
        self.blob
    }
}

#[derive(Debug)]
pub struct FileMetadata {
    pub mtime: SystemTime,
    pub mode: FileMode,
    pub size: u64,
}

// Methods on this type map 1:1 to SQL queries. rusqlite errors are handled and converted to
// sqlarfs errors.
#[derive(Debug)]
pub struct Store<'conn> {
    inner: InnerTransaction<'conn>,
}

impl<'conn> Store<'conn> {
    pub fn new(tx: rusqlite::Transaction<'conn>) -> Self {
        Self {
            inner: InnerTransaction::Transaction(tx),
        }
    }

    pub fn into_tx(self) -> rusqlite::Transaction<'conn> {
        match self.inner {
            InnerTransaction::Transaction(tx) => tx,
            // This will only ever be the case in the middle of a [`Store::exec`] block, where it's
            // not possible call this method.
            InnerTransaction::Savepoint(_) => unreachable!(),
        }
    }

    fn tx(&self) -> &rusqlite::Connection {
        match &self.inner {
            InnerTransaction::Transaction(transaction) => transaction,
            InnerTransaction::Savepoint(savepoint) => savepoint,
        }
    }

    fn savepoint(&mut self) -> crate::Result<Savepoint> {
        Ok(match &mut self.inner {
            InnerTransaction::Transaction(transaction) => transaction.savepoint()?,
            InnerTransaction::Savepoint(savepoint) => savepoint.savepoint()?,
        })
    }

    /// Execute the given function inside of a savepoint.
    pub fn exec<T, F>(&mut self, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Store) -> crate::Result<T>,
    {
        let savepoint = self.savepoint()?;

        let mut store = Store {
            inner: InnerTransaction::Savepoint(savepoint),
        };

        let result = f(&mut store)?;

        let savepoint = match store.inner {
            InnerTransaction::Savepoint(savepoint) => savepoint,
            InnerTransaction::Transaction(_) => unreachable!(),
        };

        savepoint.commit()?;

        Ok(result)
    }

    pub fn create_table(&self) -> crate::Result<()> {
        self.tx().execute(
            "
            CREATE TABLE IF NOT EXISTS sqlar(
                name TEXT PRIMARY KEY,
                mode INT,
                mtime INT,
                sz INT,
                data BLOB
            );
            ",
            (),
        )?;

        Ok(())
    }

    pub fn create_file(
        &self,
        path: &Path,
        mode: Option<FileMode>,
        mtime: Option<SystemTime>,
    ) -> crate::Result<()> {
        let unix_mtime = mtime
            .map(|mtime| -> crate::Result<_> {
                Ok(mtime
                    .duration_since(time::UNIX_EPOCH)
                    .map_err(|err| crate::Error::new(crate::ErrorKind::InvalidArgs, err))?
                    .as_secs())
            })
            .transpose()?;

        let result = self.tx().execute(
            "INSERT INTO sqlar (name, mode, mtime, sz, data) VALUES (?1, ?2, ?3, 0, zeroblob(0))",
            (
                path.to_string_lossy(),
                mode.map(|mode| mode.bits()),
                unix_mtime,
            ),
        );

        match result {
            Ok(_) => Ok(()),
            Err(err)
                if err.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) =>
            {
                Err(crate::Error::new(crate::ErrorKind::AlreadyExists, err))
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn delete_file(&self, path: &Path) -> crate::Result<()> {
        let num_updated = self.tx().execute(
            "DELETE FROM sqlar WHERE name = ?1",
            (path.to_string_lossy(),),
        )?;

        if num_updated == 0 {
            return Err(crate::ErrorKind::NotFound.into());
        }

        Ok(())
    }

    pub fn open_blob(&self, path: &Path, read_only: bool) -> crate::Result<FileBlob> {
        let row = self
            .tx()
            .query_row(
                "SELECT rowid, sz FROM sqlar WHERE name = ?1;",
                (path.to_string_lossy(),),
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        match row {
            Some((row_id, original_size)) => Ok(FileBlob {
                blob: self.tx().blob_open(
                    rusqlite::DatabaseName::Main,
                    "sqlar",
                    "data",
                    row_id,
                    read_only,
                )?,
                original_size,
            }),
            None => Err(crate::ErrorKind::NotFound.into()),
        }
    }

    pub fn allocate_blob(&self, path: &Path, len: u64) -> crate::Result<()> {
        let num_updated = self.tx().execute(
            "UPDATE sqlar SET data = zeroblob(?1) WHERE name = ?2",
            (len, path.to_string_lossy()),
        )?;

        if num_updated == 0 {
            return Err(crate::ErrorKind::NotFound.into());
        }

        Ok(())
    }

    pub fn store_blob(&self, path: &Path, bytes: &[u8]) -> crate::Result<()> {
        let num_updated = self.tx().execute(
            "UPDATE sqlar SET data = ?1 WHERE name = ?2",
            (bytes, path.to_string_lossy()),
        )?;

        if num_updated == 0 {
            return Err(crate::ErrorKind::NotFound.into());
        }

        Ok(())
    }

    pub fn set_size(&self, path: &Path, size: u64) -> crate::Result<()> {
        let num_updated = self.tx().execute(
            "UPDATE sqlar SET sz = ?1 WHERE name = ?2",
            (size, path.to_string_lossy()),
        )?;

        if num_updated == 0 {
            return Err(crate::ErrorKind::NotFound.into());
        }

        Ok(())
    }
}
