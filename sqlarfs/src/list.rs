use std::fmt;
use std::path::{Path, PathBuf};

use super::metadata::{FileMetadata, FileType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListSort {
    Size,
    Mtime,
    Depth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Options for sorting and filtering a list of files.
///
/// This is used with [`Archive::list_with`].
///
/// Unless you specify a sort criteria with [`ListOptions::by_mtime`] or [`ListOptions::by_size`],
/// the order of the returned files is unspecified.
///
/// [`Archive::list_with`]: crate::Archive::list_with
#[derive(Debug, Clone)]
pub struct ListOptions {
    pub(super) direction: Option<SortDirection>,
    pub(super) sort: Option<ListSort>,
    pub(super) ancestor: Option<PathBuf>,
    pub(super) parent: Option<PathBuf>,
    pub(super) file_type: Option<FileType>,
    pub(super) is_invalid: bool,
}

impl Default for ListOptions {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn default() -> Self {
        Self::new()
    }
}

impl ListOptions {
    /// Create a new [`ListOptions`] with default settings.
    pub fn new() -> Self {
        Self {
            direction: None,
            sort: None,
            ancestor: None,
            parent: None,
            file_type: None,
            is_invalid: false,
        }
    }

    /// Only return files that are descendants of the given `directory`.
    ///
    /// This returns all descendants, not just immediate children.
    ///
    /// This does not include the `directory` itself in the list of descendants.
    ///
    /// If `directory` is a regular file, the returned list will be empty.
    ///
    /// This is mutually exclusive with [`ListOptions::children_of`].
    pub fn descendants_of<P: AsRef<Path>>(mut self, directory: P) -> Self {
        if self.parent.is_some() {
            self.is_invalid = true;
            return self;
        }

        self.ancestor = Some(directory.as_ref().to_path_buf());

        self
    }

    /// Only return files that are immediate children of the given `directory`.
    ///
    /// This does not include the `directory` itself in the list of descendants.
    ///
    /// If `directory` is a regular file, the returned list will be empty.
    ///
    /// This is mutually exclusive with [`ListOptions::descendants_of`].
    pub fn children_of<P: AsRef<Path>>(mut self, directory: P) -> Self {
        if self.ancestor.is_some() {
            self.is_invalid = true;
            return self;
        }

        self.parent = Some(directory.as_ref().to_path_buf());

        self
    }

    /// Only return files of this [`FileType`].
    ///
    /// This is mutually exclusive with [`ListOptions::by_size`].
    pub fn file_type(mut self, file_type: FileType) -> Self {
        if self.sort == Some(ListSort::Size) {
            self.is_invalid = true;
            return self;
        }

        self.file_type = Some(file_type);

        self
    }

    /// Sort by depth in the directory tree.
    ///
    /// This ensures parents always come before their children (or children before their parents in
    /// descending mode).
    ///
    /// This is mutually exclusive with [`ListOptions::by_mtime`] and [`ListOptions::by_size`].
    pub fn by_depth(mut self) -> Self {
        if self.sort.is_some() {
            self.is_invalid = true;
            return self;
        }

        self.sort = Some(ListSort::Depth);

        self
    }

    /// Sort by last modification time.
    ///
    /// This is mutually exclusive with [`ListOptions::by_depth`] and [`ListOptions::by_size`].
    pub fn by_mtime(mut self) -> Self {
        if self.sort.is_some() {
            self.is_invalid = true;
            return self;
        }

        self.sort = Some(ListSort::Mtime);

        self
    }

    /// Sort by file size.
    ///
    /// If this is specified, then the list will only contain regular files, skipping directories
    /// and symbolic links.
    ///
    /// This is mutually exclusive with [`ListOptions::by_depth`], [`ListOptions::by_mtime`] and
    /// [`ListOptions::file_type`].
    pub fn by_size(mut self) -> Self {
        if self.sort.is_some() || self.file_type.is_some() {
            self.is_invalid = true;
            return self;
        }

        self.sort = Some(ListSort::Size);

        self
    }

    /// Sort in ascending order (the default).
    ///
    /// This is mutually exclusive with [`ListOptions::desc`].
    pub fn asc(mut self) -> Self {
        if self.direction.is_some() {
            self.is_invalid = true;
            return self;
        }

        self.direction = Some(SortDirection::Asc);

        self
    }

    /// Sort in descending order.
    ///
    /// This is mutually exclusive with [`ListOptions::asc`].
    pub fn desc(mut self) -> Self {
        if self.direction.is_some() {
            self.is_invalid = true;
            return self;
        }

        self.direction = Some(SortDirection::Desc);

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
#[derive(Debug)]
pub struct ListEntry {
    pub(super) path: PathBuf,
    pub(super) metadata: FileMetadata,
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

    /// The file metadata.
    pub fn metadata(&self) -> &FileMetadata {
        &self.metadata
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
