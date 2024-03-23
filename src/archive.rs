#[derive(Debug)]
pub struct Transaction<'a> {
    archive: &'a Sqlar,
    tx: rusqlite::Transaction<'a>,
}

impl<'a> Transaction<'a> {
    pub fn exec<T, E, F>(self, f: F) -> Result<T, E>
    where
        F: FnOnce(&Sqlar) -> Result<T, E>,
        E: From<crate::Error>,
    {
        let result = f(self.archive)?;

        self.tx.commit().map_err(crate::Error::from)?;

        Ok(result)
    }

    pub fn sqlar(&self) -> &Sqlar {
        self.archive
    }

    pub fn rollback(self) -> crate::Result<()> {
        Ok(self.tx.rollback()?)
    }

    pub fn commit(self) -> crate::Result<()> {
        Ok(self.tx.commit()?)
    }
}

/// A SQLite archive file.
#[derive(Debug)]
pub struct Sqlar {
    conn: rusqlite::Connection,
}

impl Sqlar {
    pub fn new(conn: rusqlite::Connection) -> Self {
        Self { conn }
    }

    pub fn transaction(&mut self) -> crate::Result<Transaction> {
        Ok(Transaction {
            archive: self,
            tx: self.conn.unchecked_transaction()?,
        })
    }
}
