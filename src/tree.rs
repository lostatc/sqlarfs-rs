use std::fs;
use std::io;
use std::path::Path;

use super::archive::Archive;
use super::list::ListOptions;
use super::metadata::FileType;
use super::mode::{ReadMode, WriteMode};

/// Options for archiving a filesystem directory tree to an [`Archive`].
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
            follow_symlinks: true,
            children: false,
            recursive: true,
            preserve_metadata: true,
        }
    }

    /// Follow symbolic links.
    ///
    /// If this is `false`, symbolic links will be silently skipped.
    ///
    /// The default is `true`.
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

fn read_metadata(path: &Path, follow_symlinks: bool) -> crate::Result<fs::Metadata> {
    let metadata_result = if follow_symlinks {
        fs::metadata(path)
    } else {
        fs::symlink_metadata(path)
    };

    match metadata_result {
        Ok(metadata) => Ok(metadata),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(crate::ErrorKind::NotFound.into()),
        Err(err) => Err(err.into()),
    }
}

pub fn archive_tree<T>(
    archive: &mut Archive,
    src_root: &Path,
    dest_root: &Path,
    opts: &ArchiveOptions,
    mode_adapter: &T,
) -> crate::Result<()>
where
    T: ReadMode,
{
    if dest_root == Path::new("") && !opts.children {
        return Err(crate::Error::msg(
            crate::ErrorKind::InvalidArgs,
            "Cannot use an empty path as the destination directory unless archiving the children of the source directory."
        ));
    }

    let src_is_dir = read_metadata(src_root, opts.follow_symlinks)?.is_dir();

    let mut stack = if opts.children && src_is_dir {
        fs::read_dir(src_root)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![src_root.to_path_buf()]
    };

    while let Some(path) = stack.pop() {
        let metadata = read_metadata(&path, opts.follow_symlinks)?;

        let file_type = if metadata.is_file() {
            FileType::File
        } else if metadata.is_dir() {
            FileType::Dir
        } else {
            // We ignore special files.
            continue;
        };

        let dest_path = dest_root.join(path
            .strip_prefix(src_root)
            .expect("Could not get path relative to ancestor while walking the directory tree. This is a bug.")
        );
        dbg!(&dest_path);

        let mut archive_file = archive.open(dest_path)?;

        if opts.preserve_metadata {
            let mode = mode_adapter.read_mode(&path, &metadata)?;

            // `std::fs::Metadata::modified` returns an error when mtime isn't available on the current
            // platform, in which case we just don't set the mtime in the archive.
            let mtime = metadata.modified().ok();

            // Create the file with its metadata.
            archive_file.create_with(file_type, mode, mtime)?;
        } else {
            match file_type {
                FileType::File => archive_file.create_file()?,
                FileType::Dir => archive_file.create_dir()?,
            }
        }

        match file_type {
            FileType::File => {
                // Copy the file contents.
                let mut fs_file = fs::File::open(&path)?;
                archive_file.write_file(&mut fs_file)?;
            }
            FileType::Dir if opts.recursive => {
                for entry in fs::read_dir(&path)? {
                    let entry = entry?;
                    let path = entry.path();
                    stack.push(path);
                }
            }
            _ => {}
        }
    }

    Ok(())
}

pub fn extract_tree<T>(
    archive: &mut Archive,
    src_root: &Path,
    dest_root: &Path,
    mode_adapter: &T,
) -> crate::Result<()>
where
    T: WriteMode,
{
    let list_opts = ListOptions::new().descendants_of(src_root).by_depth();

    // We need to collect this into a vector because iterating over the entries will borrow the
    // `Archive`, and we need to borrow it mutably to copy the file contents.
    let entries = archive
        .list_with(&list_opts)?
        .collect::<Result<Vec<_>, _>>()?;

    for entry in entries {
        let dest_path = dest_root.join(
            entry.path
                .strip_prefix(src_root)
                .expect("Could not get path relative to ancestor while walking the directory tree. This is a bug.")
        );

        match entry.metadata().kind {
            Some(FileType::File) => {
                let mut fs_file = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&dest_path)
                    .map_err(|err| match err.kind() {
                        io::ErrorKind::AlreadyExists => {
                            crate::Error::new(crate::ErrorKind::AlreadyExists, err)
                        }
                        _ => err.into(),
                    })?;

                if let Some(mtime) = entry.metadata().mtime {
                    fs_file.set_modified(mtime)?;
                }

                let mut archive_file = archive.open(&entry.path)?;
                let mut reader = archive_file.reader()?;

                io::copy(&mut reader, &mut fs_file)?;
            }
            Some(FileType::Dir) => {
                fs::create_dir(&dest_path).map_err(|err| match err.kind() {
                    io::ErrorKind::AlreadyExists => {
                        crate::Error::new(crate::ErrorKind::AlreadyExists, err)
                    }
                    _ => err.into(),
                })?;
            }
            // This library doesn't allow putting special files or files without a mode into the
            // archive, but if it was generated by another implementation, we have no idea what
            // kind of data might be in there.
            None => continue,
        }

        if let Some(mode) = entry.metadata().mode {
            mode_adapter.write_mode(&dest_path, mode)?;
        }
    }

    Ok(())
}
