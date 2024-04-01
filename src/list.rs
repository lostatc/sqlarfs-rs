use std::fmt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::FileMode;

/// How to sort a list of file paths.
///
/// This is used with [`ListOptions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ListSort {
    /// Sort by the file's size.
    Size,

    /// Sort by the file's mtime.
    Mtime,
}

/// Which direction to sort a list of file paths in.
///
/// This is used with [`ListOptions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// Ascending
    Asc,

    /// Descending
    Desc,
}

/// Options for sorting and filtering a list of files.
///
/// This is used with [`Archive::list_with`].
///
/// The default sort order is unspecified.
///
/// [`Archive::list_with`]: crate::Archive::list_with
#[derive(Debug, Clone)]
pub struct ListOptions {
    pub(super) direction: SortDirection,
    pub(super) sort: Option<ListSort>,
    pub(super) ancestor: Option<PathBuf>,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ListOptions {
    /// Create a new [`ListOptions`] with default settings.
    pub fn new() -> Self {
        Self {
            direction: SortDirection::Asc,
            sort: None,
            ancestor: None,
        }
    }

    /// Choose how to sort the list of files.
    pub fn sort(&mut self, sort: ListSort) -> &mut Self {
        self.sort = Some(sort);

        self
    }

    /// Choose the sort direction.
    pub fn direction(&mut self, direction: SortDirection) -> &mut Self {
        self.direction = direction;

        self
    }

    /// Only return files that are descendants of this directory.
    ///
    /// This returns all descendants, not just immediate children.
    pub fn descendants<P: AsRef<Path>>(&mut self, directory: P) -> &mut Self {
        self.ancestor = Some(directory.as_ref().to_path_buf());

        self
    }
}

/// An entry when iterating over a list of files.
///
/// You can use [`Archive::list`] and [`Archive::list_with`] to iterate over the files in an
/// archive.
///
/// [`Archive::list`]: crate::Archive::list
/// [`Archive::list_with`]: crate::Archive::list_with
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEntry {
    pub(super) path: PathBuf,
    pub(super) mode: Option<FileMode>,
    pub(super) mtime: Option<SystemTime>,
    pub(super) size: u64,
}

impl ListEntry {
    /// The file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Consume this entry and return the owned file path.
    pub fn into_path(self) -> PathBuf {
        self.path
    }

    /// The file mode (permissions).
    pub fn mode(&self) -> Option<FileMode> {
        self.mode
    }

    /// The file's last modification time.
    pub fn mtime(&self) -> Option<SystemTime> {
        self.mtime
    }

    /// The original uncompressed size of the file.
    pub fn size(&self) -> u64 {
        self.size
    }
}

pub type ListMapFunc = Box<dyn FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<ListEntry>>;

#[ouroboros::self_referencing]
struct ListEntriesInner<'conn> {
    stmt: rusqlite::Statement<'conn>,
    #[borrows(mut stmt)]
    #[covariant]
    iter: rusqlite::MappedRows<'this, ListMapFunc>,
}

impl<'conn> fmt::Debug for ListEntriesInner<'conn> {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListEntries").finish_non_exhaustive()
    }
}
fn build_list_entries_inner(
    stmt: rusqlite::Statement,
    params: Vec<Box<dyn rusqlite::ToSql>>,
    map_func: ListMapFunc,
) -> crate::Result<ListEntriesInner> {
    ListEntriesInnerTryBuilder {
        stmt,
        iter_builder: |stmt| {
            stmt.query_map(
                params
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>()
                    .as_slice(),
                map_func,
            )
            .map_err(crate::Error::from)
        },
    }
    .try_build()
}

/// An iterator over the files in an archive.
///
/// This is returned by [`Archive::list`] and [`Archive::list_with`].
///
/// [`Archive::list`]: crate::Archive::list
/// [`Archive::list_with`]: crate::Archive::list_with
#[derive(Debug)]
pub struct ListEntries<'conn> {
    inner: ListEntriesInner<'conn>,
}

impl<'conn> ListEntries<'conn> {
    pub(super) fn new(
        stmt: rusqlite::Statement<'conn>,
        params: Vec<Box<dyn rusqlite::ToSql>>,
        map_func: ListMapFunc,
    ) -> crate::Result<Self> {
        Ok(Self {
            inner: build_list_entries_inner(stmt, params, map_func)?,
        })
    }
}

impl<'conn> Iterator for ListEntries<'conn> {
    type Item = crate::Result<ListEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .with_iter_mut(|iter| iter.next())
            .map(|item| item.map_err(crate::Error::from))
    }
}
