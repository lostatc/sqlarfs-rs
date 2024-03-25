use super::archive::Archive;

/// An open transaction on an [`Archive`].
///
/// If a `Transaction` is dropped without committing, the transaction is rolled back.
#[derive(Debug)]
pub struct Transaction<'a> {
    archive: &'a Archive,
    tx: rusqlite::Transaction<'a>,
}

impl<'a> Transaction<'a> {
    pub(super) fn new(archive: &'a Archive, tx: rusqlite::Transaction<'a>) -> Self {
        Self { archive, tx }
    }

    /// Execute this transaction.
    ///
    /// This calls the given function, passing the [`Archive`] holding this transaction. If the
    /// function returns `Ok`, this transaction is committed. If the function returns `Err`, this
    /// transaction is rolled back.
    pub fn exec<T, E, F>(self, f: F) -> Result<T, E>
    where
        F: FnOnce(&Archive) -> Result<T, E>,
        E: From<crate::Error>,
    {
        let result = f(self.archive)?;

        self.tx.commit().map_err(crate::Error::from)?;

        Ok(result)
    }

    /// Get a reference to the [`Archive`] holding this transaction.
    pub fn archive(&self) -> &Archive {
        self.archive
    }

    /// Roll back this transaction.
    pub fn rollback(self) -> crate::Result<()> {
        Ok(self.tx.rollback()?)
    }

    /// Commit this transaction.
    pub fn commit(self) -> crate::Result<()> {
        Ok(self.tx.commit()?)
    }
}
