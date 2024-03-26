use std::path::Path;

use rusqlite::blob::Blob;
use rusqlite::OptionalExtension;

#[derive(Debug)]
enum InnerTransaction<'a> {
    Transaction(&'a mut rusqlite::Transaction<'a>),
    Savepoint(rusqlite::Savepoint<'a>),
}

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

    /// Execute the given function inside of a savepoint.
    pub fn exec<T, F>(&'a mut self, f: F) -> crate::Result<T>
    where
        F: FnOnce(&mut Store) -> crate::Result<T>,
    {
        let savepoint = match &mut self.inner {
            InnerTransaction::Transaction(transaction) => transaction.savepoint()?,
            InnerTransaction::Savepoint(savepoint) => savepoint.savepoint()?,
        };

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

    pub fn open_blob(&self, path: &Path, read_only: bool) -> crate::Result<Blob> {
        let row = self
            .tx()
            .query_row(
                "SELECT rowid FROM sqlar WHERE path = ?1;",
                (path.to_string_lossy(),),
                |row| row.get(0),
            )
            .optional()?;

        match row {
            Some(row_id) => Ok(self.tx().blob_open(
                rusqlite::DatabaseName::Main,
                "sqlar",
                "data",
                row_id,
                read_only,
            )?),
            None => Err(crate::Error::NotFound),
        }
    }

    pub fn truncate_blob(&self, path: &Path) -> crate::Result<()> {
        let empty_buf: &[u8] = &[];

        let num_updated = self.tx().execute(
            "UPDATE sqlar SET data = ?1 WHERE path = ?2",
            (empty_buf, path.to_string_lossy()),
        )?;

        if num_updated == 0 {
            return Err(crate::Error::NotFound);
        }

        Ok(())
    }
}
