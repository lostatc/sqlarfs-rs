use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::FileMetadata;

use super::archive::Archive;
use super::list::ListOptions;
use super::metadata::FileType;
use super::mode::{ReadMode, WriteMode};

/// Options for archiving files in the filesystem to an [`Archive`].
///
/// This is used with [`Archive::archive_with`].
///
/// [`Archive`]: crate::Archive
/// [`Archive::archive_with`]: crate::Archive::archive_with
#[derive(Debug, Clone)]
pub struct ArchiveOptions {
    follow_symlinks: bool,
    children: bool,
    recursive: bool,
    preserve_metadata: bool,
}

impl Default for ArchiveOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchiveOptions {
    /// Create a new [`ArchiveOptions`] default settings.
    pub fn new() -> Self {
        Self {
            follow_symlinks: false,
            children: false,
            recursive: true,
            preserve_metadata: true,
        }
    }

    /// Follow symbolic links.
    ///
    /// If this is `true`, the file the symbolic links points to will be archived instead of the
    /// symbolic link itself.
    ///
    /// The default is `false`.
    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    /// Archive the children of the source directory instead of the source directory itself.
    ///
    /// This puts the children of the source directory into the given destination directory.
    ///
    /// As a special case, you can use an empty path as the destination directory to put the
    /// children in the root of the archive.
    ///
    /// The default is `false`.
    pub fn children(mut self, children: bool) -> Self {
        self.children = children;
        self
    }

    /// Archive the source directory recursively.
    ///
    /// This has no effect if the source is a regular file.
    ///
    /// The default is `true`.
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Preserve file metadata when copying files into the archive.
    ///
    /// The default is `true`.
    pub fn preserve_metadata(mut self, preserve: bool) -> Self {
        self.preserve_metadata = preserve;
        self
    }
}

/// Options for extracting files in an [`Archive`] into the filesystem.
///
/// This is used with [`Archive::extract_with`].
///
/// [`Archive`]: crate::Archive
/// [`Archive::archive_with`]: crate::Archive::archive_with
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    children: bool,
    recursive: bool,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtractOptions {
    /// Create a new [`ExtractOptions`] with default settings.
    pub fn new() -> Self {
        Self {
            children: false,
            recursive: true,
        }
    }

    /// Extract the children of the source directory instead of the source directory itself.
    ///
    /// This puts the children of the source directory into the given destination directory.
    ///
    /// As a special case, you can use an empty path as the source directory to extract all files
    /// in the root of the archive.
    ///
    /// The default is `false`.
    pub fn children(mut self, children: bool) -> Self {
        self.children = children;
        self
    }

    /// Extract the source directory recursively.
    ///
    /// This has no effect if the source is a regular file.
    ///
    /// The default is `true`.
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }
}

fn read_metadata(path: &Path) -> crate::Result<fs::Metadata> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(crate::Error::FileNotFound {
            path: path.to_owned(),
        }),
        Err(err) => Err(err.into()),
    }
}

fn rebase_path(path: &Path, new_base: &Path, old_base: &Path) -> PathBuf {
    new_base.join(path.strip_prefix(old_base).expect(
        "Could not get path relative to ancestor while walking the directory tree. This is a bug.",
    ))
}

impl Archive<'_> {
    pub(super) fn archive_file<T>(
        &mut self,
        src_path: &Path,
        dest_path: &Path,
        opts: &ArchiveOptions,
        mode_adapter: &T,
        ancestor_stack: Vec<PathBuf>,
    ) -> crate::Result<()>
    where
        T: ReadMode,
    {
        let metadata = read_metadata(src_path)?;

        let file_type = if metadata.is_file() {
            FileType::File
        } else if metadata.is_dir() {
            FileType::Dir
        } else if metadata.is_symlink() {
            FileType::Symlink
        } else {
            // We ignore special files.
            return Ok(());
        };

        let mut archive_file = self.open(dest_path)?;

        match file_type {
            FileType::File => archive_file.create_file()?,
            FileType::Dir => archive_file.create_dir()?,
            FileType::Symlink => {
                let target = fs::read_link(src_path)?;

                for ancestor in &ancestor_stack {
                    if same_file::is_same_file(&target, ancestor)? {
                        return Err(crate::Error::FilesystemLoop);
                    }
                }

                if opts.follow_symlinks {
                    return self.archive_file(
                        &target,
                        dest_path,
                        opts,
                        mode_adapter,
                        ancestor_stack,
                    );
                } else {
                    archive_file.create_symlink(&target)?;
                }
            }
        }

        if opts.preserve_metadata {
            let mode = mode_adapter.read_mode(src_path, &metadata)?;
            // `std::fs::Metadata::modified` returns an error when mtime isn't available on the
            // current platform, in which case we just don't set the mtime in the archive.
            let mtime = metadata.modified().ok();

            archive_file.set_mode(Some(mode))?;
            archive_file.set_mtime(mtime)?;
        }

        match file_type {
            FileType::File => {
                // Copy the file contents.
                let mut fs_file = fs::File::open(src_path)?;
                archive_file.write_file(&mut fs_file)?;
            }
            FileType::Dir if opts.recursive => {
                for entry in fs::read_dir(src_path)? {
                    let entry_path = entry?.path();
                    let dest_path = rebase_path(&entry_path, dest_path, src_path);

                    let mut ancestor_stack = ancestor_stack.clone();
                    ancestor_stack.push(src_path.to_owned());

                    self.archive_file(&entry_path, &dest_path, opts, mode_adapter, ancestor_stack)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub(super) fn archive_tree<T>(
        &mut self,
        src_root: &Path,
        dest_root: &Path,
        opts: &ArchiveOptions,
        mode_adapter: &T,
    ) -> crate::Result<()>
    where
        T: ReadMode,
    {
        let dest_is_empty = dest_root == Path::new("");

        if dest_is_empty && !opts.children {
            return Err(crate::Error::InvalidArgs {
                reason: String::from("Cannot use an empty path as the destination directory unless archiving the children of the source directory.")
            });
        }

        if opts.children && !dest_is_empty && !self.open(dest_root)?.metadata()?.is_dir() {
            return Err(crate::Error::NotADirectory {
                path: dest_root.to_owned(),
            });
        }

        // Wrap the error to provide a more helpful error message.
        let metadata = read_metadata(src_root)?;

        let src_is_dir = metadata.is_dir();

        let paths = if opts.children && !src_is_dir {
            return Err(crate::Error::NotADirectory {
                path: src_root.to_owned(),
            });
        } else if opts.children {
            fs::read_dir(src_root)?
                .map(|entry| entry.map(|entry| entry.path()))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            vec![src_root.to_path_buf()]
        };

        for path in paths {
            let dest_path = rebase_path(&path, dest_root, src_root);
            self.archive_file(&path, &dest_path, opts, mode_adapter, Vec::new())?;
        }

        Ok(())
    }

    pub(super) fn extract_file<T>(
        &mut self,
        src_path: &Path,
        dest_path: &Path,
        metadata: &FileMetadata,
        mode_adapter: &T,
    ) -> crate::Result<()>
    where
        T: WriteMode,
    {
        match metadata {
            FileMetadata::File { mtime, mode, .. } => {
                let mut fs_file = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(dest_path)
                    .map_err(|err| {
                        // Windows will throw an `io::ErrorKind::PermissionDenied` if the file
                        // already exists and is a directory.
                        if err.kind() == io::ErrorKind::AlreadyExists
                            || (cfg!(windows) && err.kind() == io::ErrorKind::PermissionDenied)
                        {
                            crate::Error::FileAlreadyExists {
                                path: dest_path.into(),
                            }
                        } else if err.kind() == io::ErrorKind::NotFound {
                            crate::Error::NoParentDirectory {
                                path: dest_path.into(),
                            }
                        } else {
                            err.into()
                        }
                    })?;

                let mut archive_file = self.open(src_path)?;
                let mut reader = archive_file.reader()?;

                io::copy(&mut reader, &mut fs_file)?;

                if let Some(mtime) = mtime {
                    fs_file.set_modified(*mtime)?;
                }

                if let Some(mode) = mode {
                    mode_adapter.write_mode(dest_path, *mode)?;
                }
            }
            FileMetadata::Dir { mode, .. } => {
                fs::create_dir(dest_path).map_err(|err| match err.kind() {
                    io::ErrorKind::AlreadyExists => crate::Error::FileAlreadyExists {
                        path: dest_path.into(),
                    },
                    io::ErrorKind::NotFound => crate::Error::NoParentDirectory {
                        path: dest_path.into(),
                    },
                    _ => err.into(),
                })?;

                if let Some(mode) = mode {
                    mode_adapter.write_mode(dest_path, *mode)?;
                }
            }
            // We currently do not attempt to set the mtime of symlinks, because Rust doesn't seem
            // to provide a way to do that.
            FileMetadata::Symlink { target, .. } => {
                // This is a no-op on non-Unix-like systems.
                #[cfg(unix)]
                {
                    std::os::unix::fs::symlink(target, dest_path).map_err(|err| {
                        match err.kind() {
                            io::ErrorKind::AlreadyExists => crate::Error::FileAlreadyExists {
                                path: dest_path.into(),
                            },
                            io::ErrorKind::NotFound => crate::Error::NoParentDirectory {
                                path: dest_path.into(),
                            },
                            _ => err.into(),
                        }
                    })?;
                }
            }
        }

        Ok(())
    }

    pub(super) fn extract_tree<T>(
        &mut self,
        src_root: &Path,
        dest_root: &Path,
        opts: &ExtractOptions,
        mode_adapter: &T,
    ) -> crate::Result<()>
    where
        T: WriteMode,
    {
        let src_path_is_empty = src_root == Path::new("");

        if !opts.children && src_path_is_empty {
            return Err(crate::Error::InvalidArgs {
                reason: String::from("Cannot use an empty path as the source directory unless archiving the children of the source directory.")
            });
        }

        if opts.children && !src_path_is_empty && !self.open(src_root)?.metadata()?.is_dir() {
            return Err(crate::Error::NotADirectory {
                path: src_root.into(),
            });
        }

        // TODO: Reading the file metadata at this point presents a race condition, because the
        // file could change out from under us between now and when we start extracting files
        // into it. However, this might be the best we can reasonably do.
        //
        // Ideally, once `io_error_more` is stable, we match on the
        // `io::ErrorKind::NotADirectory` returned when we try to extract a file into this
        // directory.
        //
        // However, as of time of writing, that error kind is not thrown on Windows; the
        // Windows API doesn't seem to distinguish between these cases:
        //
        // 1. The parent of the dest path doesn't exist.
        // 2. The parent of the dest path exists but is not a directory.
        if opts.children {
            let metadata = read_metadata(dest_root)?;

            if !metadata.is_dir() {
                return Err(crate::Error::NotADirectory {
                    path: dest_root.into(),
                });
            }
        }

        if !opts.children {
            let src_metadata = self.open(src_root)?.metadata()?;
            self.extract_file(src_root, dest_root, &src_metadata, mode_adapter)?;
        }

        if !opts.children && !opts.recursive {
            return Ok(());
        }

        let list_opts = if opts.recursive {
            ListOptions::new().descendants_of(src_root).by_depth()
        } else {
            ListOptions::new().children_of(src_root).by_depth()
        };

        // We need to collect the entries into a vector because iterating over the entries will
        // borrow the `Archive`, and we need to borrow it mutably to copy the file contents.
        let entries = self.list_with(&list_opts)?.collect::<Result<Vec<_>, _>>()?;

        for entry in entries {
            let dest_path = rebase_path(&entry.path, dest_root, src_root);
            self.extract_file(entry.path(), &dest_path, entry.metadata(), mode_adapter)?;
        }

        Ok(())
    }
}
