use super::archive::Archive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TransactionBehavior {
    Deferred,
    Immediate,
    Exclusive,
}

#[derive(Debug)]
pub struct Transaction<'a> {
    archive: &'a Archive,
    tx: rusqlite::Transaction<'a>,
}

impl<'a> Transaction<'a> {
    pub(super) fn new(archive: &'a Archive, tx: rusqlite::Transaction<'a>) -> Self {
        Self { archive, tx }
    }

    pub fn exec<T, E, F>(self, f: F) -> Result<T, E>
    where
        F: FnOnce(&Archive) -> Result<T, E>,
        E: From<crate::Error>,
    {
        let result = f(self.archive)?;

        self.tx.commit().map_err(crate::Error::from)?;

        Ok(result)
    }

    pub fn archive(&self) -> &Archive {
        self.archive
    }

    pub fn rollback(self) -> crate::Result<()> {
        Ok(self.tx.rollback()?)
    }

    pub fn commit(self) -> crate::Result<()> {
        Ok(self.tx.commit()?)
    }
}
