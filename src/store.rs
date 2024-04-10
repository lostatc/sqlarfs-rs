use std::path::PathBuf;
use std::time::{self, Duration, SystemTime, UNIX_EPOCH};

use rusqlite::blob::Blob;
use rusqlite::{OptionalExtension, Savepoint};

use crate::list::SortDirection;

use super::list::{ListEntries, ListEntry, ListMapFunc, ListOptions, ListSort};
use super::metadata::{FileMetadata, FileMode, FileType, DIR_MODE, FILE_MODE, TYPE_MASK};
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
pub struct BlobSize {
    // The original size of the blob (the `sz` column).
    pub original: u64,

    // The actual size of the blob (the compressed size, if it's compressed).
    pub actual: u64,
}

impl BlobSize {
    pub fn is_compressed(&self) -> bool {
        self.actual != self.original
    }
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

    // Execute the given function inside of a savepoint.
    //
    // Operations that perform multiple writes to the database should wrap them with this method to
    // ensure atomicity and consistency.
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

    // The file mode is mandatory even though the column in the database is nullable because we
    // need a reliable way to determine whether the file is a directory or not, and we can't set
    // the file type bits in the mode without also setting the permissions bits because we wouldn't
    // have a way to distinguish a file with undefined permissions from a file with `0o000`
    // permissions.
    pub fn create_file(
        &self,
        path: &str,
        kind: FileType,
        mode: FileMode,
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

        let mode_bits = match kind {
            FileType::File => mode.to_file_mode(),
            FileType::Dir => mode.to_dir_mode(),
        };

        let result = self.tx().execute(
            "INSERT INTO sqlar (name, mode, mtime, sz, data) VALUES (?1, ?2, ?3, 0, zeroblob(0))",
            (path, mode_bits, unix_mtime),
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

    pub fn delete_file(&self, path: &str) -> crate::Result<()> {
        let num_updated = self
            .tx()
            .execute("DELETE FROM sqlar WHERE name = ?1", (path,))?;

        if num_updated == 0 {
            return Err(crate::ErrorKind::NotFound.into());
        }

        Ok(())
    }

    pub fn open_blob(&self, path: &str, read_only: bool) -> crate::Result<FileBlob> {
        let row = self
            .tx()
            .query_row(
                "SELECT rowid, sz FROM sqlar WHERE name = ?1;",
                (path,),
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

    pub fn allocate_blob(&self, path: &str, len: u64) -> crate::Result<()> {
        let num_updated = self.tx().execute(
            "UPDATE sqlar SET data = zeroblob(?1) WHERE name = ?2",
            (len, path),
        )?;

        if num_updated == 0 {
            return Err(crate::ErrorKind::NotFound.into());
        }

        Ok(())
    }

    pub fn store_blob(&self, path: &str, bytes: &[u8]) -> crate::Result<()> {
        let num_updated = self
            .tx()
            .execute("UPDATE sqlar SET data = ?1 WHERE name = ?2", (bytes, path))?;

        if num_updated == 0 {
            return Err(crate::ErrorKind::NotFound.into());
        }

        Ok(())
    }

    pub fn read_metadata(&self, path: &str) -> crate::Result<FileMetadata> {
        self.tx()
            .query_row(
                "SELECT mode, mtime, sz FROM sqlar WHERE name = ?1;",
                (path,),
                |row| {
                    let raw_mode = row.get::<_, Option<u32>>(0)?;

                    Ok(FileMetadata {
                        mode: raw_mode.map(FileMode::from_mode),
                        mtime: row
                            .get::<_, Option<u64>>(1)?
                            .map(|mtime_secs| UNIX_EPOCH + Duration::from_secs(mtime_secs)),
                        size: u64_from_usize(row.get(2)?),
                        kind: raw_mode.and_then(FileType::from_mode),
                    })
                },
            )
            .optional()?
            .ok_or(crate::ErrorKind::NotFound.into())
    }

    pub fn set_mode(&self, path: &str, mode: Option<FileMode>) -> crate::Result<()> {
        let num_updated = self.tx().execute(
            "UPDATE sqlar SET mode = ?1 WHERE name = ?2",
            (mode.map(|mode| mode.bits()), path),
        )?;

        if num_updated == 0 {
            return Err(crate::ErrorKind::NotFound.into());
        }

        Ok(())
    }

    pub fn set_mtime(&self, path: &str, mtime: Option<SystemTime>) -> crate::Result<()> {
        let mtime_secs = mtime
            .map(|mtime| -> crate::Result<_> {
                Ok(mtime
                    .duration_since(time::UNIX_EPOCH)
                    .map_err(|err| crate::Error::new(crate::ErrorKind::InvalidArgs, err))?
                    .as_secs())
            })
            .transpose()?;

        let num_updated = self.tx().execute(
            "UPDATE sqlar SET mtime = ?1 WHERE name = ?2",
            (mtime_secs, path),
        )?;

        if num_updated == 0 {
            return Err(crate::ErrorKind::NotFound.into());
        }

        Ok(())
    }

    pub fn set_size(&self, path: &str, size: u64) -> crate::Result<()> {
        let num_updated = self
            .tx()
            .execute("UPDATE sqlar SET sz = ?1 WHERE name = ?2", (size, path))?;

        if num_updated == 0 {
            return Err(crate::ErrorKind::NotFound.into());
        }

        Ok(())
    }

    pub fn blob_size(&self, path: &str) -> crate::Result<BlobSize> {
        self.tx()
            .query_row(
                "SELECT sz, length(data) FROM sqlar WHERE name = ?1;",
                (path,),
                |row| {
                    Ok(BlobSize {
                        original: row.get(0)?,
                        actual: row.get(1)?,
                    })
                },
            )
            .optional()?
            .ok_or(crate::ErrorKind::NotFound.into())
    }

    pub fn list_files(&self, opts: &ListOptions) -> crate::Result<ListEntries> {
        let order_column = match opts.sort {
            Some(ListSort::Size) => "sz",
            Some(ListSort::Mtime) => "mtime",
            // The contract of `Archive::list` and `Archive::list_with` is that default sort order
            // is unspecified.
            None => "rowid",
        };

        let direction = match opts.direction {
            Some(SortDirection::Asc) | None => "ASC",
            Some(SortDirection::Desc) => "DESC",
        };

        let stmt = self.tx().prepare(&format!(
            "
            SELECT
                name, mode, mtime, sz
            FROM
                sqlar
            WHERE
                iif(?1 = '', true, name GLOB ?1 || '/?*')
                AND iif(?2 = 0, true, (mode & ?2) = ?3)
            ORDER BY
                {order_column} {direction}
        "
        ))?;

        let params: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(
                opts.ancestor
                    .as_ref()
                    .map(|ancestor| ancestor.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            ),
            Box::new(if let Some(ListSort::Size) = opts.sort {
                TYPE_MASK
            } else {
                0
            }),
            Box::new(FILE_MODE),
        ];

        let map_func: ListMapFunc = Box::new(|row| {
            let raw_mode = row.get::<_, Option<u32>>(1)?;

            Ok(ListEntry {
                path: PathBuf::from(row.get::<_, String>(0)?),
                metadata: FileMetadata {
                    mode: raw_mode.map(FileMode::from_mode),
                    mtime: row
                        .get::<_, Option<u64>>(2)?
                        .map(|mtime_secs| UNIX_EPOCH + Duration::from_secs(mtime_secs)),
                    size: row.get(3)?,
                    kind: raw_mode.and_then(FileType::from_mode),
                },
            })
        });

        ListEntries::new(stmt, params, map_func)
    }

    pub fn has_dir_mode(&self, path: &str) -> crate::Result<bool> {
        let result = self.tx().query_row(
            "SELECT name FROM sqlar WHERE name = ?1 AND (mode & ?2) = ?3",
            (path, TYPE_MASK, DIR_MODE),
            |_| Ok(()),
        );

        match result {
            Ok(_) => Ok(true),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    pub fn has_descendants(&self, path: &str) -> crate::Result<bool> {
        let result = self.tx().query_row(
            "SELECT name FROM sqlar WHERE name GLOB ?1 || '/?*' LIMIT 1",
            (path,),
            |_| Ok(()),
        );

        match result {
            Ok(_) => Ok(true),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    pub fn has_nonzero_size_ancestor(&self, path: &str) -> crate::Result<bool> {
        let result = self.tx().query_row(
            "SELECT name FROM sqlar WHERE ?1 GLOB name || '/?*' AND sz > 0 LIMIT 1",
            (path,),
            |_| Ok(()),
        );

        match result {
            Ok(_) => Ok(true),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }
}
