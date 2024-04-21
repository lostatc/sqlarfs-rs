use std::fs;
use std::io;
use std::path::Path;

use super::archive::Archive;
use super::mode::{ReadMode, WriteMode};

/// Options for archiving a filesystem directory tree to an [`Archive`].
///
/// This is used with [`Archive::archive`].
///
/// [`Archive`]: crate::Archive
/// [`Archive::archive`]: crate::Archive::archive
#[derive(Debug)]
#[non_exhaustive]
pub struct ArchiveOptions {
    /// Follow symbolic links.
    ///
    /// If this is `false`, symbolic links will be silently skipped.
    ///
    /// The default is `true`.
    follow_symlinks: bool,
}

impl Default for ArchiveOptions {
    fn default() -> Self {
        Self {
            follow_symlinks: true,
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
    let metadata = read_metadata(src_root, opts.follow_symlinks)?;

    // When `std::io::ErrorKind::NotADirectory` is stable, we can catch that error when it's
    // returned by `std::fs::read_dir` instead of reading the metadata first. As written, this
    // presents a race condition, as the file might change from a non-directory to a directory
    // between now and when we try to iterate over its children.
    if !metadata.is_dir() {
        return Err(crate::Error::msg(
            crate::ErrorKind::NotADirectory,
            "Cannot archive a file that is not a directory.",
        ));
    }

    let mut stack = if dest_root == Path::new("") {
        // Archive the children of `src_root` into the root of the archive.
        fs::read_dir(src_root)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        // Archive `src_root` to `dest_root`.
        vec![src_root.to_path_buf()]
    };

    while let Some(path) = stack.pop() {
        let metadata = read_metadata(&path, opts.follow_symlinks)?;

        let file_type = if metadata.is_file() {
            crate::FileType::File
        } else if metadata.is_dir() {
            crate::FileType::Dir
        } else {
            // We ignore special files.
            continue;
        };

        let relative_path = match path.strip_prefix(src_root) {
            Ok(path) => path,
            Err(_) => {
                panic!("Could not get path relative to ancestor while walking the directory tree. This is a bug.")
            }
        };

        let dest_path = dest_root.join(relative_path);

        let mode = mode_adapter.read_mode(&path, &metadata)?;

        let mut archive_file = archive.open(dest_path)?;
        archive_file.create_with(file_type, mode, metadata.modified().ok())?;

        match file_type {
            crate::FileType::File => {
                let mut fs_file = fs::File::open(&path)?;
                archive_file.write_file(&mut fs_file)?;
            }
            crate::FileType::Dir => {
                for entry in fs::read_dir(&path)? {
                    let entry = entry?;
                    let path = entry.path();
                    stack.push(path);
                }
            }
        }
    }

    Ok(())
}
