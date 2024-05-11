use std::path::PathBuf;
use std::time::{self, Duration, SystemTime, UNIX_EPOCH};

use rusqlite::blob::Blob;
use rusqlite::{OptionalExtension, Savepoint};

use crate::list::SortDirection;
use crate::metadata::SYMLINK_MODE;

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

    pub fn create_table(&self, fail_if_exists: bool) -> crate::Result<()> {
        self.tx().execute(
            if fail_if_exists {
                "
                CREATE TABLE sqlar(
                    name TEXT PRIMARY KEY,
                    mode INT,
                    mtime INT,
                    sz INT,
                    data BLOB
                );
                "
            } else {
                "
                CREATE TABLE IF NOT EXISTS sqlar(
                    name TEXT PRIMARY KEY,
                    mode INT,
                    mtime INT,
                    sz INT,
                    data BLOB
                );
                "
            },
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
        symlink_target: Option<&str>,
    ) -> crate::Result<()> {
        if symlink_target.is_some() && kind != FileType::Symlink {
            panic!("Tried to create a non-symlink with a symlink target. This is a bug.");
        }

        let unix_mtime = mtime
            .map(|mtime| -> crate::Result<_> {
                Ok(mtime
                    .duration_since(time::UNIX_EPOCH)
                    .map_err(|err| crate::Error::InvalidArgs {
                        reason: err.to_string(),
                    })?
                    .as_secs())
            })
            .transpose()?;

        // While we don't rely on this information to determine the file type, we set the correct
        // file mode bits when we create a file, because that's what the reference implementation
        // does.
        let mode_bits = match kind {
            FileType::File => mode.to_file_mode(),
            FileType::Dir => mode.to_dir_mode(),
            FileType::Symlink => mode.to_symlink_mode(),
        };

        let initial_size = match kind {
            FileType::File | FileType::Dir => 0,
            // The negative size indicates that the file is a symlink.
            FileType::Symlink => -1,
        };

        let initial_data: Option<Box<dyn rusqlite::ToSql>> = match kind {
            FileType::File => Some(Box::<Vec<u8>>::default()),
            // A NULL value in the `data` column indicates that the file is a directory.
            FileType::Dir => None,
            FileType::Symlink => Some(Box::new(
                symlink_target.expect("Tried to create a symlink without a target. This is a bug."),
            )),
        };

        let result = self.tx().execute(
            "INSERT INTO sqlar (name, mode, mtime, sz, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            (path, mode_bits, unix_mtime, initial_size, initial_data),
        );

        match result {
            Ok(_) => Ok(()),
            Err(err)
                if err.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) =>
            {
                Err(crate::Error::FileAlreadyExists { path: path.into() })
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn delete_file(&self, path: &str) -> crate::Result<()> {
        // Deleting files must be recursive so that the archive doesn't end up with orphan files.
        let num_updated = self.tx().execute(
            "DELETE FROM sqlar WHERE name = ?1 OR name GLOB ?1 || '/?*'",
            (path,),
        )?;

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
                "
                SELECT
                    mode,
                    mtime,
                    sz,
                    iif(sz < 0, data, NULL) AS target,
                    data IS NULL AS is_dir
                FROM
                    sqlar
                WHERE
                    name = ?1;
                ",
                (path,),
                |row| {
                    let mode = row.get::<_, Option<u32>>(0)?.map(FileMode::from_mode);
                    let mtime = row
                        .get::<_, Option<u64>>(1)?
                        .map(|mtime_secs| UNIX_EPOCH + Duration::from_secs(mtime_secs));
                    let size: i64 = row.get(2)?;
                    // When the `data` column contains a symlink target, its type is `TEXT`, not
                    // `BLOB`. Remember that columns in SQLite are dynamically typed.
                    let symlink_target: Option<String> = row.get(3)?;
                    let is_dir: bool = row.get(4)?;

                    // We ignore the file mode in the database when determining the file type.
                    Ok(if let Some(target) = symlink_target {
                        FileMetadata::Symlink {
                            mtime,
                            target: PathBuf::from(target),
                        }
                    } else if is_dir {
                        FileMetadata::Dir { mode, mtime }
                    } else {
                        FileMetadata::File {
                            mode,
                            mtime,
                            size: size.try_into().expect("The file size in the database was negative, but we should have already checked for this. This is a bug."),
                        }
                    })
                },
            )
            .optional()?
            .ok_or(crate::Error::FileNotFound { path: path.into() })
    }

    pub fn set_mode(&self, path: &str, mode: Option<FileMode>) -> crate::Result<()> {
        // If the file is a symlink, this is a no-op. Symlinks always have 777 permissions.
        let num_updated = self.tx().execute(
            "UPDATE sqlar SET mode = iif(mode & ?1 = ?2, mode, mode & ?1 | ?3) WHERE name = ?4",
            (TYPE_MASK, SYMLINK_MODE, mode.map(|mode| mode.bits()), path),
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
                    .map_err(|err| crate::Error::InvalidArgs {
                        reason: err.to_string(),
                    })?
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
            Some(ListSort::Size) => "s.sz",
            Some(ListSort::Mtime) => "s.mtime",
            Some(ListSort::Depth) => "p.segments",
            // The contract of `Archive::list` and `Archive::list_with` is that default sort order
            // is unspecified.
            None => "s.rowid",
        };

        let direction = match opts.direction {
            Some(SortDirection::Asc) | None => "ASC",
            Some(SortDirection::Desc) => "DESC",
        };

        let stmt = self.tx().prepare(&format!(
            "
            WITH path_segments AS (
                SELECT
                    name,
                    length(name) - length(replace(name, '/', '')) AS segments
                FROM
                    sqlar
            )
            SELECT
                s.name,
                s.mode,
                s.mtime,
                s.sz,
                iif(s.sz = -1, s.data, NULL) AS target,
                s.data IS NULL AS is_dir
            FROM
                sqlar AS s
            JOIN
                path_segments AS p ON s.name = p.name
            WHERE
                iif(?1 IS NULL OR ?1 = '', true, s.name GLOB ?1 || '/?*')
                AND iif(?3 IS NULL, true, (s.mode & ?2) = ?3)
                AND iif(?4 IS NULL, true, (s.mode & ?2) = ?4)
                AND CASE
                    WHEN ?5 IS NULL THEN true
                    WHEN ?5 = '' THEN NOT s.name GLOB '*/*'
                    ELSE s.name GLOB ?5 || '/?*' AND NOT s.name GLOB ?5 || '/?*/*'
                END
            ORDER BY
                {order_column} {direction}
        "
        ))?;

        let params: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(
                opts.ancestor
                    .as_ref()
                    .map(|ancestor| ancestor.to_string_lossy().into_owned()),
            ),
            Box::new(TYPE_MASK),
            Box::new(if let Some(ListSort::Size) = opts.sort {
                Some(FILE_MODE)
            } else {
                None
            }),
            Box::new(match opts.file_type {
                Some(FileType::File) => Some(FILE_MODE),
                Some(FileType::Dir) => Some(DIR_MODE),
                Some(FileType::Symlink) => Some(SYMLINK_MODE),
                None => None,
            }),
            Box::new(
                opts.parent
                    .as_ref()
                    .map(|parent| parent.to_string_lossy().into_owned()),
            ),
        ];

        let map_func: ListMapFunc = Box::new(|row| {
            let mode = row.get::<_, Option<u32>>(1)?.map(FileMode::from_mode);
            let mtime = row
                .get::<_, Option<u64>>(2)?
                .map(|mtime_secs| UNIX_EPOCH + Duration::from_secs(mtime_secs));
            let size: i64 = row.get(3)?;
            // When the `data` column contains a symlink target, its type is `TEXT`, not `BLOB`.
            // Remember that columns in SQLite are dynamically typed.
            let symlink_target: Option<String> = row.get(4)?;
            let is_dir: bool = row.get(5)?;

            let metadata = if let Some(target) = symlink_target {
                FileMetadata::Symlink {
                    mtime,
                    target: PathBuf::from(target),
                }
            } else if is_dir {
                FileMetadata::Dir { mode, mtime }
            } else {
                FileMetadata::File {
                    mode,
                    mtime,
                    size: size.try_into().expect("The file size in the database was negative, but we should have already checked for this. This is a bug."),
                }
            };

            Ok(ListEntry {
                path: PathBuf::from(row.get::<_, String>(0)?),
                metadata,
            })
        });

        ListEntries::new(stmt, params, map_func)
    }
}
