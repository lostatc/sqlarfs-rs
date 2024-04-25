use std::fs;
use std::io;
use std::path::Path;

use super::archive::Archive;
use super::metadata::FileType;
use super::mode::{ReadMode, WriteMode};

/// Options for archiving a filesystem directory tree to an [`Archive`].
///
/// This is used with [`Archive::archive_with`].
///
/// [`Archive`]: crate::Archive
/// [`Archive::archive_with`]: crate::Archive::archive_with
#[derive(Debug)]
#[non_exhaustive]
pub struct ArchiveOptions {
    /// Follow symbolic links.
    ///
    /// If this is `false`, symbolic links will be silently skipped.
    ///
    /// The default is `true`.
    pub dereference: bool,

    /// Archive the children of the source directory instead of the source directory itself.
    ///
    /// This puts the children of the source directory into the given destination directory.
    ///
    /// As a special case, you can use an empty path as the destination directory to put the
    /// children in the root of the archive.
    ///
    /// The default is `false`.
    pub children: bool,

    /// Archive the source directory recursively.
    ///
    /// This has no effect if the source is a regular file.
    ///
    /// The default is `true`.
    pub recursive: bool,

    /// Preserve file metadata when copying files into the archive.
    ///
    /// The default is `true`.
    pub preserve: bool,
}

impl Default for ArchiveOptions {
    fn default() -> Self {
        Self {
            dereference: true,
            children: false,
            recursive: true,
            preserve: true,
        }
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

// TODO: Think real hard about whether you want this to work like `cp` and what the behavior should
// be when an empty path is passed for `dest_root`.
pub fn archive_tree<T>(
    archive: &mut Archive,
    src_root: &Path,
    dest_root: &Path,
    opts: &ArchiveOptions,
    mode_adapter: &T,
) -> crate::Result<()>
where
    T: ReadMode + WriteMode,
{
    let src_is_dir = read_metadata(src_root, opts.dereference)?.is_dir();

    let mut stack = if opts.children && src_is_dir {
        fs::read_dir(src_root)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![src_root.to_path_buf()]
    };

    while let Some(path) = stack.pop() {
        let metadata = read_metadata(&path, opts.dereference)?;

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

        if opts.preserve {
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
