use std::path::{Path, PathBuf};

/// How to sort a list of file paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SortCritera {
    /// The file's size.
    Size,

    /// The file's mtime.
    Mtime,
}

/// Which direction to sort a list of file paths in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// Ascending
    Asc,

    /// Descending
    Desc,
}

/// Options for sorting and filtering a list of files.
///
/// This is used with [`File::list_with`].
///
/// The default sort order is unspecified.
#[derive(Debug, Clone)]
pub struct ListOptions {
    direction: SortDirection,
    sort: Option<SortCritera>,
    ancestor: Option<PathBuf>,
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
    pub fn sort(&mut self, criteria: SortCritera) -> &mut Self {
        self.sort = Some(criteria);

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
    pub fn parent<P: AsRef<Path>>(&mut self, directory: P) -> &mut Self {
        self.ancestor = Some(directory.as_ref().to_path_buf());

        self
    }
}
