use std::path::Path;
use std::time::{self, SystemTime};

use rusqlite::blob::Blob;
use rusqlite::{OptionalExtension, Savepoint};

use super::metadata::FileMode;
use super::util::u64_from_usize;

const EMPTY_BLOB: &[u8] = &[];

#[derive(Debug)]
enum InnerTransaction<'a> {
    Transaction(&'a mut rusqlite::Transaction<'a>),
    Savepoint(rusqlite::Savepoint<'a>),
}

pub struct FileBlob<'a> {
    blob: Blob<'a>,
    original_size: u64,
}

impl<'a> FileBlob<'a> {
    pub fn is_compressed(&self) -> bool {
        u64_from_usize(self.blob.len()) != self.original_size
    }

    pub fn into_blob(self) -> Blob<'a> {
        self.blob
    }
}

// Methods on this type map 1:1 to SQL queries. rusqlite errors are handled and converted to
// sqlarfs errors.
#[derive(Debug)]
pub struct Store<'a> {
    inner: InnerTransaction<'a>,
}

impl<'a> Store<'a> {
    pub fn new(tx: &'a mut rusqlite::Transaction<'a>) -> Self {
        Self {
            inner: InnerTransaction::Transaction(tx),
        }
    }

    fn tx(&self) -> &rusqlite::Connection {
        match &self.inner {
            InnerTransaction::Transaction(transaction) => transaction,
            InnerTransaction::Savepoint(savepoint) => savepoint,
        }
    }

    fn savepoint(&'a mut self) -> crate::Result<Savepoint<'a>> {
        Ok(match &mut self.inner {
            InnerTransaction::Transaction(transaction) => transaction.savepoint()?,
            InnerTransaction::Savepoint(savepoint) => savepoint.savepoint()?,
        })
    }

    /// Execute the given function inside of a savepoint.
    pub fn exec<T, F>(&'a mut self, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Store) -> crate::Result<T>,
    {
        let savepoint = self.savepoint()?;

        let mut store = Self {
            inner: InnerTransaction::Savepoint(savepoint),
        };

        let result = f(&mut store)?;

        match store.inner {
            InnerTransaction::Savepoint(savepoint) => savepoint.commit()?,
            InnerTransaction::Transaction(_) => unreachable!(),
        }

        Ok(result)
    }

    pub fn create_file(
        &mut self,
        path: &Path,
        mode: FileMode,
        mtime: SystemTime,
    ) -> crate::Result<()> {
        let unix_mtime = mtime
            .duration_since(time::UNIX_EPOCH)
            .map_err(|_| crate::Error::InvalidArgs)?
            .as_secs();

        let result = self.tx().execute(
            "INSERT INTO sqlar (path, mode, mtime, sz, data) VALUES (?1, ?2, ?3, 0, ?4)",
            (path.to_string_lossy(), mode.bits(), unix_mtime, EMPTY_BLOB),
        );

        match result {
            Ok(_) => Ok(()),
            Err(err)
                if err.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) =>
            {
                Err(crate::Error::AlreadyExists)
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn open_blob(&self, path: &Path, read_only: bool) -> crate::Result<FileBlob> {
        let row = self
            .tx()
            .query_row(
                "SELECT rowid, sz FROM sqlar WHERE path = ?1;",
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
            None => Err(crate::Error::NotFound),
        }
    }

    pub fn truncate_blob(&self, path: &Path) -> crate::Result<()> {
        let num_updated = self.tx().execute(
            "UPDATE sqlar SET data = ?1 WHERE path = ?2",
            (EMPTY_BLOB, path.to_string_lossy()),
        )?;

        if num_updated == 0 {
            return Err(crate::Error::NotFound);
        }

        Ok(())
    }
}
